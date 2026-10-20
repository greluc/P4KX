use std::error::Error;
use std::io::{self, Write};
use std::path::Path;

use p4kx_extraction::{EntryFailure, ExtractionReport};
use serde::Serialize;

use crate::cli::Conflict;
use crate::dcb_export;
use crate::json;
use crate::verify::{MismatchJson, write_mismatch};
#[derive(Serialize)]
pub(crate) struct ReportJson<'a> {
    target: String,
    extracted: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    converted: Option<usize>,
    skipped: &'a [String],
    failed: Vec<FailureJson<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mismatched: Option<Vec<MismatchJson<'a>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unconverted: Option<Vec<FailureJson<'a>>>,
    cancelled: &'a [String],
    archive_changed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    records: Option<RecordsJson<'a>>,
}

#[derive(Serialize)]
struct RecordsJson<'a> {
    exported: usize,
    skipped: &'a [String],
    failed: Vec<dcb_export::FailureJson<'a>>,
    cancelled: usize,
    error_values: usize,
    records_with_errors: usize,
}

#[derive(Serialize)]
struct FailureJson<'a> {
    path: &'a str,
    error: String,
}

pub(crate) fn report_json<'a>(
    report: &'a ExtractionReport,
    target: &Path,
    verify: bool,
) -> ReportJson<'a> {
    ReportJson {
        target: json::path(target),
        extracted: report.extracted(),
        converted: report.converted(),
        skipped: report.skipped(),
        failed: failures_json(report.failures(), verify),
        mismatched: verify.then(|| {
            report
                .failures()
                .iter()
                .filter_map(MismatchJson::of)
                .collect()
        }),
        unconverted: report
            .converted()
            .map(|_| failures_json(report.unconverted(), false)),
        cancelled: report.cancelled(),
        archive_changed: report.archive_changed(),
        records: report.records().map(|records| RecordsJson {
            exported: records.written(),
            skipped: records.skipped(),
            failed: records
                .failures()
                .iter()
                .map(dcb_export::failure_json)
                .collect(),
            cancelled: records.cancelled(),
            error_values: records.error_values(),
            records_with_errors: records.records_with_errors(),
        }),
    }
}

fn failures_json(failures: &[EntryFailure], without_mismatches: bool) -> Vec<FailureJson<'_>> {
    failures
        .iter()
        .filter(|failure| !(without_mismatches && failure.checksum_mismatch().is_some()))
        .map(|failure| FailureJson {
            path: failure.path(),
            error: reason(failure),
        })
        .collect()
}

/// Returns the error of `failure` followed by its causes, separated by colons.
pub(crate) fn reason(failure: &EntryFailure) -> String {
    let mut reason = failure.error().to_string();
    let mut source = failure.error().source();
    while let Some(cause) = source {
        reason.push_str(": ");
        reason.push_str(&cause.to_string());
        source = cause.source();
    }
    reason
}

pub(crate) fn print_report(
    report: &ExtractionReport,
    target: &Path,
    conflict: Conflict,
    verify: bool,
) -> io::Result<()> {
    let mut err = io::stderr().lock();
    for failure in report.failures() {
        if failure.checksum_mismatch().is_some() {
            write_mismatch(&mut err, failure)?;
        } else {
            writeln!(err, "failed {}: {}", failure.path(), reason(failure))?;
        }
    }
    for unconverted in report.unconverted() {
        writeln!(
            err,
            "not converted {}: {}",
            unconverted.path(),
            reason(unconverted)
        )?;
    }
    let mut out = io::stdout().lock();
    let kept = if conflict == Conflict::IfChanged {
        "unchanged"
    } else {
        "already exists"
    };
    for path in report.skipped() {
        writeln!(out, "skipped {path}: {kept}")?;
    }
    for path in report.cancelled() {
        writeln!(out, "cancelled {path}")?;
    }
    let converted = match (report.converted(), report.unconverted().len()) {
        (None, _) => String::new(),
        (Some(count), 0) => format!(" ({count} converted)"),
        (Some(count), unconverted) => format!(" ({count} converted, {unconverted} not converted)"),
    };
    let bad = report.checksum_mismatches().count();
    let (failed, mismatches) = if verify {
        (
            report.failures().len() - bad,
            format!(", checksum mismatches {bad}"),
        )
    } else {
        (report.failures().len(), String::new())
    };
    writeln!(
        out,
        "extracted {}{converted}, skipped {}, failed {failed}{mismatches}, cancelled {} into {}",
        report.extracted(),
        report.skipped().len(),
        report.cancelled().len(),
        target.display()
    )
}
