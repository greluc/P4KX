use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, ExitCode};
use std::time::SystemTime;

use anyhow::{Context, Result};
use p4kx_channels::channel_name;
use p4kx_extraction::{
    Archive, ArchiveWatch, Build, CancelToken, ConflictPolicy, Entry, EntryFailure, ExtractOptions,
    ExtractionReport, Filter, Progress, create_default_target, extract, select_entries,
};
use serde::Serialize;
use thiserror::Error;

use crate::cli::{Conflict, ExtractArgs};
use crate::exit;
use crate::json;
use crate::progress::ProgressLine;

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
    cancelled: &'a [String],
    archive_changed: bool,
}

#[derive(Serialize)]
struct FailureJson<'a> {
    path: &'a str,
    error: String,
}

/// Extracts the requested Entries, showing progress, stopping on Ctrl+C and quitting on a second
/// Ctrl+C and stopping when the Archive changes on disk, and prints the report: as JSON, or the
/// skipped and cancelled Entries and a summary to stdout and the failed Entries and a change of
/// the Archive to stderr.
pub(crate) fn run(args: &ExtractArgs, as_json: bool) -> Result<ExitCode> {
    let archive = Archive::open(&args.archive)?;
    let watch = ArchiveWatch::new(&archive);
    let entries = requested_entries(args, &archive)?;
    let target = prepare_target(args, &archive)?;
    let cancel = CancelToken::new();
    let on_interrupt = cancel.clone();
    ctrlc::set_handler(move || {
        if on_interrupt.is_cancelled() {
            process::exit(exit::INTERRUPTED.into());
        }
        on_interrupt.cancel();
    })
    .context("cannot install the Ctrl+C handler")?;
    let line = ProgressLine::new();
    let on_progress = |progress: Progress| {
        if args
            .cancel_after
            .is_some_and(|count| progress.entries_done >= count)
        {
            cancel.cancel();
        }
        line.show(progress);
    };
    let options = ExtractOptions {
        policy: policy(args.conflict),
        cancel: Some(&cancel),
        progress: Some(&on_progress),
        watch: Some(&watch),
    };
    let report = extract(&archive, &entries, &target, &options);
    line.finish();
    let printed = if as_json {
        json::print(&report_json(&report, &target))
    } else {
        print_report(&report, &target, args.conflict)
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

const fn exit_code(report: &ExtractionReport) -> u8 {
    if report.archive_changed() {
        exit::ARCHIVE_CHANGED
    } else if report.is_complete() {
        exit::SUCCESS
    } else {
        exit::PARTIAL_FAILURE
    }
}

const fn policy(conflict: Conflict) -> ConflictPolicy {
    match conflict {
        Conflict::Skip => ConflictPolicy::Skip,
        Conflict::Overwrite => ConflictPolicy::Overwrite,
        Conflict::IfChanged => ConflictPolicy::IfChanged,
    }
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
        archive_changed: report.archive_changed(),
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

fn print_report(report: &ExtractionReport, target: &Path, conflict: Conflict) -> io::Result<()> {
    let mut err = io::stderr().lock();
    for failure in report.failures() {
        writeln!(err, "failed {}: {}", failure.path(), reason(failure))?;
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
    writeln!(
        out,
        "extracted {}, skipped {}, failed {}, cancelled {} into {}",
        report.extracted(),
        report.skipped().len(),
        report.failures().len(),
        report.cancelled().len(),
        target.display()
    )
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;

    use p4kx_extraction::{Archive, ArchiveWatch, ExtractOptions, extract};
    use p4kx_testkit::{ArchiveBuilder, TestEntry};

    use super::exit_code;
    use crate::exit;

    fn builder() -> ArchiveBuilder {
        ArchiveBuilder::new().entry(TestEntry::stored("Data\\readme.txt", "hello"))
    }

    fn extract_after(change: bool) -> Result<u8, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("Data.p4k");
        builder().write_to(&path)?;
        let archive = Archive::open(&path)?;
        let watch = ArchiveWatch::new(&archive);
        if change {
            builder()
                .entry(TestEntry::stored("Data\\patch.txt", "new"))
                .write_to(&path)?;
        }
        let target = dir.path().join("out");
        fs::create_dir(&target)?;
        let entries: Vec<_> = archive.entries().iter().collect();
        let options = ExtractOptions {
            watch: Some(&watch),
            ..ExtractOptions::default()
        };
        Ok(exit_code(&extract(&archive, &entries, &target, &options)))
    }

    #[test]
    fn an_unchanged_archive_ends_with_success() -> Result<(), Box<dyn Error>> {
        assert_eq!(extract_after(false)?, exit::SUCCESS, "nothing changed");
        Ok(())
    }

    #[test]
    fn a_changed_archive_ends_with_its_own_exit_code() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            extract_after(true)?,
            exit::ARCHIVE_CHANGED,
            "the Archive was rewritten after it was opened"
        );
        Ok(())
    }
}
