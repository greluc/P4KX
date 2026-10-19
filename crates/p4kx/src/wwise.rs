use std::io::{self, BufWriter, Write};

use anyhow::Result;
use p4kx_assembly::audio::{
    ArchiveAudio, IndexedSoundbank, LinkBreak, PathStep, SoundVariant, TriggerSounds, WemLocation,
};
use p4kx_extraction::Archive;
use thiserror::Error;

use crate::cli::{WwiseAction, WwiseArgs, WwiseResolveArgs};
use crate::entity_audio::{issues_json, warn};
use crate::json;
use crate::query_table::one_line;
use crate::wwise_json::{self, Counts, ResolveJson};

/// No ATL definition of the Archive has the requested audio trigger.
#[derive(Debug, Error)]
#[error("no ATL definition has the audio trigger {0}")]
pub(crate) struct UnknownTrigger(String);

/// Runs a `p4kx wwise` subcommand.
pub(crate) fn run(args: &WwiseArgs, as_json: bool) -> Result<()> {
    match &args.action {
        WwiseAction::Resolve(args) => resolve(args, as_json),
    }
}

/// Prints every sound variant of an audio trigger and every broken link.
fn resolve(args: &WwiseResolveArgs, as_json: bool) -> Result<()> {
    let archive = Archive::open(&args.archive)?;
    let audio = ArchiveAudio::new(&archive);
    let index = audio.index();
    let sounds = audio
        .resolve_trigger(&args.trigger)
        .ok_or_else(|| UnknownTrigger(args.trigger.clone()))?;
    let counts = Counts::of(&sounds);
    if as_json {
        json::print(&ResolveJson {
            archive: json::path(archive.path()),
            trigger: wwise_json::trigger(&sounds),
            variants: sounds.variants().iter().map(wwise_json::variant).collect(),
            breaks: sounds.breaks().iter().map(wwise_json::link_break).collect(),
            counts,
            issues: issues_json(index),
        })?;
        return Ok(());
    }
    warn(index)?;
    match print_text(&sounds, counts) {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(error.into()),
        _ => Ok(()),
    }
}

fn print_text(sounds: &TriggerSounds<'_>, counts: Counts) -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    let header = format!(
        "{}  {}",
        sounds.trigger().name(),
        sounds.definition_path().unwrap_or_default()
    );
    writeln!(out, "{}", one_line(&header))?;
    for variant in sounds.variants() {
        writeln!(out, "  {}", one_line(&variant_line(variant)))?;
    }
    for link in sounds.breaks() {
        writeln!(out, "  ! {}", one_line(&break_line(link)))?;
    }
    writeln!(
        out,
        "{} variants: {} located, {} not located, {} plug-in; {} broken links",
        counts.variants, counts.located, counts.unlocated, counts.plugins, counts.breaks
    )?;
    out.flush()
}

fn variant_line(variant: &SoundVariant<'_>) -> String {
    let location = match variant.location() {
        Some(WemLocation::Soundbank {
            soundbank,
            offset,
            len,
        }) => format!("in {} at {offset} ({len} bytes)", soundbank_name(soundbank)),
        Some(_) => "located".to_owned(),
        None => "not located".to_owned(),
    };
    let media = variant.wem_id().map_or_else(
        || format!("plug-in source {}, no WEM", variant.source().source_id),
        |wem| format!("WEM {wem} {}, {location}", variant.source_kind().name()),
    );
    format!(
        "{} in {}: {media}; {}",
        variant.event().name(),
        soundbank_name(variant.soundbank()),
        path_text(variant.path())
    )
}

fn break_line(link: &LinkBreak<'_>) -> String {
    let bank = link
        .soundbank()
        .map_or_else(String::new, |bank| format!(" in {}", soundbank_name(bank)));
    let object = link
        .object()
        .map_or_else(String::new, |object| format!(" at {}", step_text(object)));
    format!(
        "{}{bank}: broken{object}: {}; {}",
        link.event().name(),
        link.reason(),
        path_text(link.path())
    )
}

fn soundbank_name(bank: &IndexedSoundbank) -> String {
    bank.language().map_or_else(
        || bank.file_name().to_owned(),
        |language| format!("{} ({language})", bank.file_name()),
    )
}

fn path_text(path: &[PathStep]) -> String {
    path.iter()
        .map(|step| step_text(*step))
        .collect::<Vec<_>>()
        .join(" > ")
}

fn step_text(step: PathStep) -> String {
    format!("{} {}", step.kind.name(), step.id)
}
