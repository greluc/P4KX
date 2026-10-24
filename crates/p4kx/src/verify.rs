// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};
use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::{
    Archive, ArchiveWatch, Entry, Filter, VerificationReport, VerifyOptions, verify,
};
use serde::Serialize;

use crate::cli::VerifyArgs;
use crate::extract::{self, NoMatch};
use crate::failure_report::{FailureJson, MismatchJson, reason, unreadable, write_mismatch};
use crate::progress::ProgressLine;
use crate::{exit, json};

#[derive(Serialize)]
struct ReportJson<'a> {
    archive: String,
    verified: usize,
    mismatched: Vec<MismatchJson<'a>>,
    failed: Vec<FailureJson<'a>>,
    cancelled: &'a [String],
    archive_changed: bool,
}

/// Checks the Entries of the Archive, or those the Filter matches, against their checksums and
/// prints every mismatch; exits with 1 when an Entry mismatches or cannot be read.
pub(crate) fn run(args: &VerifyArgs, as_json: bool) -> Result<ExitCode> {
    let archive = crate::key::open(&args.archive)?;
    crate::issues::warn(&archive)?;
    let entries = selected(&archive, args.filter.as_deref())?;
    execute(&archive, &entries, as_json)
}

/// Returns the Entries of `archive` the Filter `text` matches, or all of them without a Filter.
///
/// # Errors
///
/// Returns [`NoMatch`] when the Filter matches none.
pub(crate) fn selected<'a>(archive: &'a Archive, text: Option<&str>) -> Result<Vec<&'a Entry>> {
    let Some(text) = text else {
        return Ok(archive.entries().iter().collect());
    };
    let entries = Filter::new(text).select(archive);
    if entries.is_empty() {
        return Err(NoMatch::new(text).into());
    }
    Ok(entries)
}

/// Verifies `entries` of `archive`, which may be a container, and prints the report.
pub(crate) fn execute(archive: &Archive, entries: &[&Entry], as_json: bool) -> Result<ExitCode> {
    crate::key::require(archive, entries)?;
    let watch = ArchiveWatch::new(archive);
    let cancel = extract::cancel_on_interrupt()?;
    let line = ProgressLine::new();
    let on_progress = |progress| line.show_verifying(progress);
    let options = VerifyOptions {
        cancel: Some(&cancel),
        progress: Some(&on_progress),
        watch: Some(&watch),
    };
    let report = verify(archive, entries, &options);
    line.finish();
    let printed = if as_json {
        json::print(&report_json(archive, &report))
    } else {
        print_report(archive, &report)
    };
    if let Err(error) = printed
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(error.into());
    }
    if report.archive_changed()
        && let Err(changed) = watch.check()
    {
        let _ignored = writeln!(io::stderr().lock(), "error: {changed}");
    }
    Ok(ExitCode::from(exit_code(&report)))
}

const fn exit_code(report: &VerificationReport) -> u8 {
    if report.archive_changed() {
        exit::ARCHIVE_CHANGED
    } else if report.is_complete() {
        exit::SUCCESS
    } else {
        exit::PARTIAL_FAILURE
    }
}

fn report_json<'a>(archive: &Archive, report: &'a VerificationReport) -> ReportJson<'a> {
    ReportJson {
        archive: json::path(archive.location()),
        verified: report.verified(),
        mismatched: report
            .failures()
            .iter()
            .filter_map(MismatchJson::of)
            .collect(),
        failed: unreadable(report.failures()).map(FailureJson::of).collect(),
        cancelled: report.cancelled(),
        archive_changed: report.archive_changed(),
    }
}

fn print_report(archive: &Archive, report: &VerificationReport) -> io::Result<()> {
    let mut err = io::stderr().lock();
    for failure in report.failures() {
        match failure.checksum_mismatch() {
            Some(_) => write_mismatch(&mut err, failure)?,
            None => writeln!(err, "failed {}: {}", failure.path(), reason(failure))?,
        }
    }
    let mut out = io::stdout().lock();
    for path in report.cancelled() {
        writeln!(out, "cancelled {path}")?;
    }
    let mismatched = report.checksum_mismatches().count();
    writeln!(
        out,
        "verified {}, mismatched {mismatched}, failed {}, cancelled {} in {}",
        report.verified(),
        report.failures().len() - mismatched,
        report.cancelled().len(),
        archive.location().display()
    )
}
