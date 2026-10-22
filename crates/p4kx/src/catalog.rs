use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use p4kx_export::{
    CatalogError, CatalogFailure, CatalogFile, CatalogOptions, CatalogSummary, Compression,
    CraftingWarning, DECOMPRESSED_LIMIT, Filter, Playability, SizeWarning, StarmapSummary,
    UPLOAD_LIMIT, catalog_schema, write_catalog,
};
use p4kx_extraction::Archive;
use thiserror::Error;

use super::catalog_json::summary_json;
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

impl InvalidExportTime {
    /// Returns the error for the export time `seconds`.
    pub(crate) const fn of(seconds: u64) -> Self {
        Self(seconds)
    }
}

/// `p4kx catalog` was given no Archive or no output file.
#[derive(Debug, Error)]
#[error("p4kx catalog needs an Archive and --out")]
pub(crate) struct MissingCatalogArgs;

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
    crate::guard::ensure_writable(out)?;
    let written = write_file(&archive, &options, out);
    line.finish();
    let summary = &written?;
    print_notes(summary.text_notes());
    print_failures(summary.failures())?;
    print_warnings(summary.warnings())?;
    warn_about_size(summary)?;
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

/// Builds the catalog and streams it into a temporary file next to `out`, compressed when the name
/// ends in `.zst`, then moves it over `out`; a failure or an interrupt leaves `out` as it was.
fn write_file(
    archive: &Archive,
    options: &CatalogOptions<'_>,
    out: &Path,
) -> Result<CatalogSummary> {
    let written = write_catalog(archive, options, Compression::for_path(out), || {
        CatalogFile::create(out).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("cannot write {}: {error}", out.display()),
            )
        })
    });
    let (summary, file) = written.map_err(into_anyhow)?;
    file.commit()
        .with_context(|| format!("cannot write {}", out.display()))?;
    Ok(summary)
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

fn print_warnings(warnings: &[CraftingWarning]) -> io::Result<()> {
    let mut err = io::stderr().lock();
    for warning in warnings {
        writeln!(err, "warning: {warning}")?;
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
    let mut err = io::stderr().lock();
    if let Some(warning) = summary.size_warning() {
        writeln!(
            err,
            "warning: the catalog file is {} bytes, {} the basetool upload limit of {UPLOAD_LIMIT} bytes",
            summary.upload_size(),
            relation(warning)
        )?;
    }
    if let Some(warning) = summary.decompressed_warning() {
        writeln!(
            err,
            "warning: the catalog is {} bytes decompressed, {} the basetool limit of {DECOMPRESSED_LIMIT} bytes for decompressed content",
            summary.size(),
            relation(warning)
        )?;
    }
    Ok(())
}

const fn relation(warning: SizeWarning) -> &'static str {
    match warning {
        SizeWarning::NearLimit => "close to",
        SizeWarning::OverLimit => "over",
    }
}

fn print_starmap(out: &mut impl Write, starmap: StarmapSummary) -> io::Result<()> {
    writeln!(
        out,
        "starmap placement: {} of {} objects have a position, {} are placed apart, {} systems, {} containers unreadable, {} missing, {} children malformed",
        starmap.placed,
        starmap.objects,
        starmap.ambiguous,
        starmap.systems,
        starmap.unreadable_containers,
        starmap.missing_containers,
        starmap.malformed_children
    )
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
    let paints = summary.paints();
    writeln!(
        stdout,
        "paint assignment: {} of {} paints fit a ship, {} fit none, {} ships have paints, {} vehicle files unreadable",
        paints.assigned, paints.paints, paints.unassigned, paints.ships, paints.unreadable_vehicles
    )?;
    print_starmap(&mut stdout, summary.starmap())?;
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
        "wrote {}: {}, {} Records skipped, {} failed",
        out.display(),
        size_text(summary),
        summary.skipped(),
        summary.failures().len()
    )
}

fn size_text(summary: &CatalogSummary) -> String {
    summary.compressed_size().map_or_else(
        || format!("{} bytes", summary.size()),
        |compressed| {
            format!(
                "{compressed} bytes zstd-compressed from {} bytes",
                summary.size()
            )
        },
    )
}
