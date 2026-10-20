use std::fmt::Write as _;
use std::io::{self, BufWriter, Write};

use anyhow::Result;
use p4kx_assembly::Entities;
use p4kx_assembly::audio::{
    ArchiveAudio, AudioIndex, EntityAudio, EntityTrigger, EventMatch, IndexedSoundbank,
    TriggerSounds, Unmatched, trigger_references,
};
use p4kx_extraction::{Archive, DataForgeEntry, Record};
use serde::Serialize;

use crate::cli::EntityAudioArgs;
use crate::json;
use crate::query_table::one_line;
use crate::wwise::{break_line, language_choice, variant_line};
use crate::wwise_json::{self, BreakJson, LanguageJson, VariantJson};

const CLASS_PREFIX: &str = "EntityClassDefinition.";

#[derive(Serialize)]
struct AudioJson<'a> {
    archive: String,
    entry: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<LanguageJson<'a>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    sounds: Option<SoundsJson<'a>>,
}

#[derive(Serialize)]
struct SoundsJson<'a> {
    variants: Vec<VariantJson<'a>>,
    breaks: Vec<BreakJson<'a>>,
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
pub(crate) struct IssueJson<'a> {
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
    let choice = language_choice(index, &args.language)?;
    let resolved = index.resolve(references);
    let sounds: Vec<Option<TriggerSounds<'_>>> = if args.sounds {
        resolved
            .triggers()
            .iter()
            .map(|trigger| audio.resolve_trigger_in(trigger.name(), &choice))
            .collect()
    } else {
        Vec::new()
    };
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
            language: args.sounds.then(|| LanguageJson::of(&choice)),
            entity: entity_json(&entity),
            triggers: resolved
                .triggers()
                .iter()
                .enumerate()
                .map(|(position, trigger)| {
                    trigger_json(trigger, sounds.get(position).and_then(Option::as_ref))
                })
                .collect(),
            counts,
            issues: issues_json(index),
        })?;
        return Ok(());
    }
    warn(index)?;
    match print_text(&entity, &resolved, &sounds, counts) {
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

fn trigger_json<'a>(
    trigger: &'a EntityTrigger<'a>,
    sounds: Option<&'a TriggerSounds<'a>>,
) -> TriggerJson<'a> {
    TriggerJson {
        sounds: sounds.map(|sounds| SoundsJson {
            variants: sounds.variants().iter().map(wwise_json::variant).collect(),
            breaks: sounds
                .breaks()
                .iter()
                .map(wwise_json::broken_link)
                .collect(),
        }),
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

pub(crate) fn issues_json(index: &AudioIndex) -> Vec<IssueJson<'_>> {
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

pub(crate) fn warn(index: &AudioIndex) -> io::Result<()> {
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

fn print_text(
    entity: &Record<'_>,
    audio: &EntityAudio<'_>,
    sounds: &[Option<TriggerSounds<'_>>],
    counts: Counts,
) -> io::Result<()> {
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
    for (position, trigger) in audio.triggers().iter().enumerate() {
        writeln!(out, "  {}", one_line(&trigger_line(trigger)))?;
        if let Some(Some(sounds)) = sounds.get(position) {
            for variant in sounds.variants() {
                writeln!(out, "    {}", one_line(&variant_line(variant)))?;
            }
            for link in sounds.breaks() {
                writeln!(out, "    ! {}", one_line(&break_line(link)))?;
            }
        }
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
        format!("{} in no soundbank", event.event().name())
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
