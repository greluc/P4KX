use std::fmt::Write as _;
use std::io::{self, BufWriter, Write};

use anyhow::Result;
use p4kx_assembly::Entities;
use p4kx_assembly::audio::{
    ArchiveAudio, AudioIndex, EntityAudio, EntityTrigger, EventMatch, IndexedSoundbank, Unmatched,
    trigger_references,
};
use p4kx_extraction::{Archive, DataForgeEntry, Record};
use serde::Serialize;

use crate::cli::EntityAudioArgs;
use crate::json;
use crate::query_table::one_line;

const CLASS_PREFIX: &str = "EntityClassDefinition.";

#[derive(Serialize)]
struct AudioJson<'a> {
    archive: String,
    entry: &'a str,
    entity: EntityJson<'a>,
    triggers: Vec<TriggerJson<'a>>,
    counts: Counts,
    issues: Vec<IssueJson<'a>>,
}

#[derive(Serialize)]
struct EntityJson<'a> {
    guid: String,
    record_path: &'a str,
    name: &'a str,
    class_name: &'a str,
}

#[derive(Serialize)]
struct TriggerJson<'a> {
    name: &'a str,
    location: &'a str,
    references: u32,
    definition: Option<&'a str>,
    events: Vec<EventJson<'a>>,
    unmatched: Option<UnmatchedJson>,
}

#[derive(Serialize)]
struct EventJson<'a> {
    name: &'a str,
    id: u32,
    localised: bool,
    soundbanks: Vec<SoundbankJson<'a>>,
}

#[derive(Serialize)]
struct SoundbankJson<'a> {
    path: &'a str,
    language: Option<&'a str>,
}

#[derive(Serialize)]
struct UnmatchedJson {
    kind: &'static str,
    message: String,
}

#[derive(Serialize)]
struct IssueJson<'a> {
    path: &'a str,
    kind: &'static str,
    message: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct Counts {
    triggers: usize,
    matched: usize,
    unmatched: usize,
}

/// Prints the audio triggers of an Entity with their Wwise events and soundbanks.
pub(crate) fn run(args: &EntityAudioArgs, as_json: bool) -> Result<()> {
    let archive = Archive::open(&args.archive)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let entity = Entities::new(&forge).find(&args.entity)?;
    let references = trigger_references(&entity.tree()?);
    let audio = ArchiveAudio::new(&archive);
    let index = audio.index();
    let resolved = index.resolve(references);
    let counts = Counts {
        triggers: resolved.triggers().len(),
        matched: resolved.matched_count(),
        unmatched: resolved
            .triggers()
            .len()
            .saturating_sub(resolved.matched_count()),
    };
    if as_json {
        json::print(&AudioJson {
            archive: json::path(archive.path()),
            entry: entry.path(),
            entity: entity_json(&entity),
            triggers: resolved.triggers().iter().map(trigger_json).collect(),
            counts,
            issues: issues_json(index),
        })?;
        return Ok(());
    }
    warn(index)?;
    match print_text(&entity, &resolved, counts) {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(error.into()),
        _ => Ok(()),
    }
}

fn entity_json<'a>(entity: &Record<'a>) -> EntityJson<'a> {
    let name = entity.name().unwrap_or_default();
    EntityJson {
        guid: entity.guid().to_string(),
        record_path: entity.record_path().unwrap_or_default(),
        name,
        class_name: name.strip_prefix(CLASS_PREFIX).unwrap_or(name),
    }
}

fn trigger_json<'a>(trigger: &'a EntityTrigger<'a>) -> TriggerJson<'a> {
    TriggerJson {
        name: trigger.name(),
        location: trigger.reference().location(),
        references: trigger.reference().count(),
        definition: trigger.definition_path(),
        events: trigger
            .events()
            .map(|event| EventJson {
                name: event.event().name(),
                id: event.event().id(),
                localised: event.event().is_localised(),
                soundbanks: event
                    .soundbanks()
                    .map(|bank| SoundbankJson {
                        path: bank.path(),
                        language: bank.language(),
                    })
                    .collect(),
            })
            .collect(),
        unmatched: trigger.unmatched().map(|reason| UnmatchedJson {
            kind: reason.kind(),
            message: reason.to_string(),
        }),
    }
}

fn issues_json(index: &AudioIndex) -> Vec<IssueJson<'_>> {
    index
        .issues()
        .iter()
        .map(|issue| IssueJson {
            path: issue.path(),
            kind: issue.error().kind(),
            message: issue.error().to_string(),
        })
        .collect()
}

fn warn(index: &AudioIndex) -> io::Result<()> {
    let mut err = io::stderr().lock();
    for issue in index.issues() {
        writeln!(
            err,
            "warning: {}",
            one_line(&format!("{}: {}", issue.path(), issue.error()))
        )?;
    }
    Ok(())
}

fn print_text(entity: &Record<'_>, audio: &EntityAudio<'_>, counts: Counts) -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    let header = entity_json(entity);
    writeln!(
        out,
        "{}",
        one_line(&format!(
            "{}  {}  {}",
            header.class_name, header.guid, header.record_path
        ))
    )?;
    for trigger in audio.triggers() {
        writeln!(out, "  {}", one_line(&trigger_line(trigger)))?;
    }
    writeln!(
        out,
        "{} triggers: {} matched, {} unmatched",
        counts.triggers, counts.matched, counts.unmatched
    )?;
    out.flush()
}

fn trigger_line(trigger: &EntityTrigger<'_>) -> String {
    let events: Vec<String> = trigger.events().map(|event| event_text(&event)).collect();
    let mut line = format!("{}:", trigger.name());
    if !events.is_empty() {
        line.push(' ');
        line.push_str(&events.join("; "));
    }
    match trigger.unmatched() {
        None => {}
        Some(reason @ (Unmatched::NoDefinition | Unmatched::NoEvent)) => {
            let _infallible = write!(line, " unmatched: {reason}");
        }
        Some(reason) => {
            let _infallible = write!(line, " (unmatched: {reason})");
        }
    }
    line
}

fn event_text(event: &EventMatch<'_>) -> String {
    let banks: Vec<String> = event.soundbanks().map(bank_text).collect();
    if banks.is_empty() {
        event.event().name().to_owned()
    } else {
        format!("{} in {}", event.event().name(), banks.join(", "))
    }
}

fn bank_text(bank: &IndexedSoundbank) -> String {
    bank.language().map_or_else(
        || bank.file_name().to_owned(),
        |language| format!("{} ({language})", bank.file_name()),
    )
}
