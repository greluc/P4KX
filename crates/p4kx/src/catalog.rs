use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use p4kx_export::{
    CatalogError, CatalogOptions, CatalogSummary, Filter, SCHEMA_VERSION, SizeWarning,
    UPLOAD_LIMIT, build_catalog, catalog_schema,
};
use p4kx_extraction::Archive;
use serde::Serialize;
use thiserror::Error;

use crate::cli::{CatalogArgs, CatalogCommand};
use crate::extract::cancel_on_interrupt;
use crate::json;
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
struct SummaryJson {
    archive: String,
    out: String,
    schema_version: u32,
    size: u64,
    upload_limit: u64,
    size_warning: Option<&'static str>,
    kinds: Vec<KindJson>,
    skipped: usize,
}

#[derive(Serialize)]
struct KindJson {
    kind: &'static str,
    count: usize,
    skipped: usize,
    complete: bool,
}

/// Writes the Basetool catalog of the Archive to the output file and prints its summary, or
/// prints the catalog's JSON Schema.
pub(crate) fn run(args: &CatalogArgs, as_json: bool) -> Result<()> {
    if matches!(args.command, Some(CatalogCommand::Schema)) {
        return print_schema();
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
    let archive = Archive::open(archive)?;
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
    warn_about_size(summary)?;
    fs::write(out, built.json()).with_context(|| format!("cannot write {}", out.display()))?;
    let printed = if as_json {
        json::print(&summary_json(&archive, out, summary))
    } else {
        print_summary(summary, out)
    };
    match printed {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(error.into()),
        _ => Ok(()),
    }
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
        CatalogError::DataForge(error) => error.into(),
        CatalogError::Record { guid, source } => {
            anyhow::Error::from(source).context(format!("cannot read the Record {guid}"))
        }
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

fn summary_json(archive: &Archive, out: &Path, summary: &CatalogSummary) -> SummaryJson {
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
    writeln!(
        stdout,
        "wrote {}: {} bytes, {} Records skipped",
        out.display(),
        summary.size(),
        summary.skipped()
    )
}
