use std::error::Error;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::{
    Archive, DataForgeEntry, Filter, Record, RecordExportOptions, RecordExportReport,
    RecordFailure, export_records,
};
use serde::Serialize;
use thiserror::Error;

use crate::cli::{Conflict, DcbExportArgs, RecordFormat};
use crate::exit;
use crate::extract::{cancel_on_interrupt, policy, prepare_target};
use crate::json;

/// A Filter that matches no Record of the DataForge.
#[derive(Debug, Error)]
#[error("no Record matches {0}")]
pub(crate) struct NoRecordMatch(String);

#[derive(Serialize)]
struct ReportJson<'a> {
    archive: String,
    entry: &'a str,
    format: &'static str,
    target: String,
    exported: usize,
    skipped: &'a [String],
    failed: Vec<FailureJson<'a>>,
    cancelled: usize,
    error_values: usize,
    records_with_errors: usize,
}

#[derive(Serialize)]
struct FailureJson<'a> {
    guid: String,
    path: &'a str,
    error: String,
}

/// Writes the selected Records of the Archive's DataForge as Record XML into the target folder
/// and prints the report.
pub(crate) fn run(args: &DcbExportArgs, as_json: bool) -> Result<ExitCode> {
    match args.format {
        RecordFormat::Xml => {}
    }
    let archive = Archive::open(&args.archive)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let records: Vec<Record<'_>> = match &args.filter {
        None => forge.records().collect(),
        Some(text) => {
            let filter = Filter::new(text);
            let selected: Vec<_> = forge
                .records()
                .filter(|record| record.record_path().is_ok_and(|path| filter.matches(path)))
                .collect();
            if selected.is_empty() {
                return Err(NoRecordMatch(text.clone()).into());
            }
            selected
        }
    };
    let target = prepare_target(args.out.as_deref(), &archive)?;
    let cancel = cancel_on_interrupt()?;
    let options = RecordExportOptions {
        policy: policy(args.conflict),
        cancel: Some(&cancel),
    };
    let report = export_records(&forge, &records, &target, &options);
    let printed = if as_json {
        json::print(&ReportJson {
            archive: json::path(archive.path()),
            entry: entry.path(),
            format: "xml",
            target: json::path(&target),
            exported: report.written(),
            skipped: report.skipped(),
            failed: report.failures().iter().map(failure_json).collect(),
            cancelled: report.cancelled(),
            error_values: report.error_values(),
            records_with_errors: report.records_with_errors(),
        })
    } else {
        print_report(&report, &target, args.conflict)
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

fn failure_json(failure: &RecordFailure) -> FailureJson<'_> {
    FailureJson {
        guid: failure.guid().to_string(),
        path: failure.record_path(),
        error: reason(failure),
    }
}

fn reason(failure: &RecordFailure) -> String {
    let mut reason = failure.error().to_string();
    let mut source = failure.error().source();
    while let Some(cause) = source {
        reason.push_str(": ");
        reason.push_str(&cause.to_string());
        source = cause.source();
    }
    reason
}

fn print_report(report: &RecordExportReport, target: &Path, conflict: Conflict) -> io::Result<()> {
    let mut err = io::stderr().lock();
    for failure in report.failures() {
        writeln!(
            err,
            "failed {} ({}): {}",
            failure.record_path(),
            failure.guid(),
            reason(failure)
        )?;
    }
    let mut out = io::stdout().lock();
    let kept = if conflict == Conflict::IfChanged {
        "unchanged"
    } else {
        "already exists"
    };
    for file in report.skipped() {
        writeln!(out, "skipped {file}: {kept}")?;
    }
    writeln!(
        out,
        "exported {} Records, skipped {}, failed {}, cancelled {} into {}; {} error values in {} Records",
        report.written(),
        report.skipped().len(),
        report.failures().len(),
        report.cancelled(),
        target.display(),
        report.error_values(),
        report.records_with_errors()
    )
}
