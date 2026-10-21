use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use p4kx_export::{
    CatalogError, CatalogFailure, CatalogOptions, CatalogSummary, Filter, Playability,
    PlayabilityCounts, SCHEMA_VERSION, SizeWarning, UPLOAD_LIMIT, build_catalog, catalog_schema,
};
use p4kx_extraction::Archive;
use serde::Serialize;
use thiserror::Error;

use crate::cli::{CatalogArgs, CatalogCommand};
use crate::exit;
use crate::extract::cancel_on_interrupt;
use crate::json;
use crate::loc::print_notes;
use crate::progress::ProgressLine;

/// An export time that no system time can hold.
#[derive(Debug, Error)]
#[error("the export time {0} lies beyond the supported range")]
pub(crate) struct InvalidExportTime(u64);

/// `p4kx catalog` was given no Archive or no output file.
#[derive(Debug, Error)]
#[error("p4kx catalog needs an Archive and --out")]
pub(crate) struct MissingCatalogArgs;

#[derive(Serialize)]
struct SummaryJson<'a> {
    archive: String,
    out: String,
    schema_version: u32,
    size: u64,
    upload_limit: u64,
    size_warning: Option<&'static str>,
    kinds: Vec<KindJson>,
    skipped: usize,
    failed: Vec<FailureJson<'a>>,
    item_classes: ItemClassJson,
    playability: PlayabilityJson,
    texts: Vec<TextJson>,
    unresolved: usize,
}

#[derive(Serialize)]
struct FailureJson<'a> {
    guid: String,
    path: &'a str,
    error: &'a str,
}

#[derive(Serialize)]
struct ItemClassJson {
    classified: usize,
    other: usize,
    missing: usize,
}

#[derive(Serialize)]
struct PlayabilityJson {
    items: PlayabilityCountsJson,
    ships: PlayabilityCountsJson,
}

#[derive(Serialize)]
struct PlayabilityCountsJson {
    player: usize,
    ai: usize,
    mission: usize,
    special: usize,
    unclassified: usize,
}

impl From<PlayabilityCounts> for PlayabilityCountsJson {
    fn from(counts: PlayabilityCounts) -> Self {
        Self {
            player: counts.of(Playability::Player),
            ai: counts.of(Playability::Ai),
            mission: counts.of(Playability::Mission),
            special: counts.of(Playability::Special),
            unclassified: counts.unclassified(),
        }
    }
}

#[derive(Serialize)]
struct TextJson {
    language: &'static str,
    available: bool,
    resolved: usize,
    unresolved: usize,
    placeholder: usize,
}

#[derive(Serialize)]
struct KindJson {
    kind: &'static str,
    count: usize,
    skipped: usize,
    complete: bool,
}

/// Writes the Basetool catalog of the Archive to the output file and prints its summary, or
/// prints the catalog's JSON Schema; the exit code is 1 when a selected Record could not be read.
pub(crate) fn run(args: &CatalogArgs, as_json: bool) -> Result<ExitCode> {
    if matches!(args.command, Some(CatalogCommand::Schema)) {
        print_schema()?;
        return Ok(ExitCode::from(exit::SUCCESS));
    }
    let (Some(archive), Some(out)) = (&args.archive, &args.out) else {
        return Err(MissingCatalogArgs.into());
    };
    let exported_at = match args.exported_at {
        None => SystemTime::now(),
        Some(seconds) => UNIX_EPOCH
            .checked_add(Duration::from_secs(seconds))
            .ok_or(InvalidExportTime(seconds))?,
    };
    let archive = crate::key::open(archive)?;
    let filter = args.filter.as_deref().map(Filter::new);
    let cancel = cancel_on_interrupt()?;
    let line = ProgressLine::new();
    let show = |progress| line.show_records(progress);
    let options = CatalogOptions {
        filter: filter.as_ref(),
        exported_at,
        cancel: Some(&cancel),
        progress: Some(&show),
    };
    let built = build_catalog(&archive, &options);
    line.finish();
    let built = built.map_err(into_anyhow)?;
    let summary = built.summary();
    print_notes(summary.text_notes());
    print_failures(summary.failures())?;
    warn_about_size(summary)?;
    crate::guard::ensure_writable(out)?;
    fs::write(out, built.json()).with_context(|| format!("cannot write {}", out.display()))?;
    let printed = if as_json {
        json::print(&summary_json(&archive, out, summary))
    } else {
        print_summary(summary, out)
    };
    if let Err(error) = printed
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(error.into());
    }
    Ok(ExitCode::from(if summary.is_complete() {
        exit::SUCCESS
    } else {
        exit::PARTIAL_FAILURE
    }))
}

fn print_failures(failures: &[CatalogFailure]) -> io::Result<()> {
    let mut err = io::stderr().lock();
    for failure in failures {
        writeln!(
            err,
            "failed {} ({}): {}",
            failure.record_path, failure.guid, failure.reason
        )?;
    }
    Ok(())
}

fn print_schema() -> Result<()> {
    let mut text = serde_json::to_string_pretty(&catalog_schema())?;
    text.push('\n');
    io::stdout().lock().write_all(text.as_bytes())?;
    Ok(())
}

fn into_anyhow(error: CatalogError) -> anyhow::Error {
    match error {
        CatalogError::Load(error) => error.into(),
        CatalogError::LoadTexts(error) => error.into(),
        CatalogError::DataForge(error) => error.into(),
        other => other.into(),
    }
}

fn warn_about_size(summary: &CatalogSummary) -> io::Result<()> {
    let Some(warning) = summary.size_warning() else {
        return Ok(());
    };
    let relation = match warning {
        SizeWarning::NearLimit => "close to",
        SizeWarning::OverLimit => "over",
    };
    writeln!(
        io::stderr().lock(),
        "warning: the catalog is {} bytes, {relation} the basetool upload limit of {UPLOAD_LIMIT} bytes",
        summary.size()
    )
}

const fn warning_name(warning: SizeWarning) -> &'static str {
    match warning {
        SizeWarning::NearLimit => "near_limit",
        SizeWarning::OverLimit => "over_limit",
    }
}

fn summary_json<'a>(archive: &Archive, out: &Path, summary: &'a CatalogSummary) -> SummaryJson<'a> {
    SummaryJson {
        archive: json::path(archive.path()),
        out: json::path(out),
        schema_version: SCHEMA_VERSION,
        size: summary.size(),
        upload_limit: UPLOAD_LIMIT,
        size_warning: summary.size_warning().map(warning_name),
        kinds: summary
            .kinds()
            .iter()
            .map(|kind| KindJson {
                kind: kind.kind.name(),
                count: kind.count,
                skipped: kind.skipped,
                complete: kind.complete,
            })
            .collect(),
        skipped: summary.skipped(),
        failed: summary
            .failures()
            .iter()
            .map(|failure| FailureJson {
                guid: failure.guid.to_string(),
                path: &failure.record_path,
                error: &failure.reason,
            })
            .collect(),
        item_classes: ItemClassJson {
            classified: summary.item_classes().classified,
            other: summary.item_classes().other,
            missing: summary.item_classes().missing,
        },
        playability: PlayabilityJson {
            items: summary.playability().items.into(),
            ships: summary.playability().ships.into(),
        },
        texts: summary
            .texts()
            .iter()
            .map(|text| TextJson {
                language: text.language.code(),
                available: text.available,
                resolved: text.resolved,
                unresolved: text.unresolved,
                placeholder: text.placeholder,
            })
            .collect(),
        unresolved: summary.unresolved(),
    }
}

fn print_summary(summary: &CatalogSummary, out: &Path) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    for kind in summary.kinds() {
        writeln!(
            stdout,
            "{}: {} written, {} skipped, {}",
            kind.kind,
            kind.count,
            kind.skipped,
            if kind.complete { "complete" } else { "partial" }
        )?;
    }
    let classes = summary.item_classes();
    writeln!(
        stdout,
        "item classes: {} read, {} other values, {} without a Class: line or English description",
        classes.classified, classes.other, classes.missing
    )?;
    let playability = summary.playability();
    for (kind, counts) in [("items", playability.items), ("ships", playability.ships)] {
        writeln!(
            stdout,
            "{kind} playability: {} player, {} ai, {} mission, {} special, {} unclassified",
            counts.of(Playability::Player),
            counts.of(Playability::Ai),
            counts.of(Playability::Mission),
            counts.of(Playability::Special),
            counts.unclassified()
        )?;
    }
    for text in summary.texts() {
        writeln!(
            stdout,
            "{}: {} texts resolved, {} unresolved, {} placeholders{}",
            text.language,
            text.resolved,
            text.unresolved,
            text.placeholder,
            if text.available {
                ""
            } else {
                ", the Archive holds no global.ini"
            }
        )?;
    }
    writeln!(
        stdout,
        "wrote {}: {} bytes, {} Records skipped, {} failed",
        out.display(),
        summary.size(),
        summary.skipped(),
        summary.failures().len()
    )
}
