use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::time::SystemTime;

use anyhow::{Context, Result};
use p4kx_channels::channel_name;
use p4kx_extraction::{
    Archive, Build, Entry, EntryFailure, ExtractionReport, Filter, create_default_target, extract,
    select_entries,
};
use serde::Serialize;
use thiserror::Error;

use crate::cli::ExtractArgs;
use crate::exit;
use crate::json;

/// A Filter that matches no Entry of the Archive.
#[derive(Debug, Error)]
#[error("no Entry matches {0}")]
pub(crate) struct NoMatch(String);

#[derive(Serialize)]
struct ReportJson<'a> {
    target: String,
    extracted: usize,
    skipped: &'a [String],
    failed: Vec<FailureJson<'a>>,
    cancelled: usize,
}

#[derive(Serialize)]
struct FailureJson<'a> {
    path: &'a str,
    error: String,
}

/// Extracts the requested Entries and prints the report: as JSON, or the skipped Entries and a
/// summary to stdout and the failed Entries to stderr.
pub(crate) fn run(args: &ExtractArgs, as_json: bool) -> Result<ExitCode> {
    let archive = Archive::open(&args.archive)?;
    let entries = requested_entries(args, &archive)?;
    let target = prepare_target(args, &archive)?;
    let report = extract(&archive, &entries, &target, &AtomicBool::new(false));
    let printed = if as_json {
        json::print(&report_json(&report, &target))
    } else {
        print_report(&report, &target)
    };
    if let Err(error) = printed
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(error.into());
    }
    Ok(ExitCode::from(if report.is_complete() {
        exit::SUCCESS
    } else {
        exit::PARTIAL_FAILURE
    }))
}

fn requested_entries<'a>(args: &ExtractArgs, archive: &'a Archive) -> Result<Vec<&'a Entry>> {
    let Some(text) = &args.filter else {
        return Ok(select_entries(archive, &args.entries)?);
    };
    let entries = Filter::new(text).select(archive);
    if entries.is_empty() {
        return Err(NoMatch(text.clone()).into());
    }
    Ok(entries)
}

fn prepare_target(args: &ExtractArgs, archive: &Archive) -> Result<PathBuf> {
    if let Some(out) = &args.out {
        fs::create_dir_all(out)
            .with_context(|| format!("cannot create the target folder {}", out.display()))?;
        return Ok(out.clone());
    }
    let parent = std::env::current_dir().context("cannot read the current directory")?;
    let channel = channel_name(archive.path());
    let version = archive.build().map(Build::version);
    create_default_target(&parent, channel.as_deref(), version, SystemTime::now())
        .with_context(|| format!("cannot create a target folder in {}", parent.display()))
}

fn report_json<'a>(report: &'a ExtractionReport, target: &Path) -> ReportJson<'a> {
    ReportJson {
        target: json::path(target),
        extracted: report.extracted(),
        skipped: report.skipped(),
        failed: report
            .failures()
            .iter()
            .map(|failure| FailureJson {
                path: failure.path(),
                error: reason(failure),
            })
            .collect(),
        cancelled: report.cancelled(),
    }
}

fn reason(failure: &EntryFailure) -> String {
    let mut reason = failure.error().to_string();
    let mut source = failure.error().source();
    while let Some(cause) = source {
        reason.push_str(": ");
        reason.push_str(&cause.to_string());
        source = cause.source();
    }
    reason
}

fn print_report(report: &ExtractionReport, target: &Path) -> io::Result<()> {
    let mut err = io::stderr().lock();
    for failure in report.failures() {
        writeln!(err, "failed {}: {}", failure.path(), reason(failure))?;
    }
    let mut out = io::stdout().lock();
    for path in report.skipped() {
        writeln!(out, "skipped {path}: already exists")?;
    }
    writeln!(
        out,
        "extracted {}, skipped {}, failed {}, cancelled {} into {}",
        report.extracted(),
        report.skipped().len(),
        report.failures().len(),
        report.cancelled(),
        target.display()
    )
}
