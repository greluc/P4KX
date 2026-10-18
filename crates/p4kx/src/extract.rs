use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::time::SystemTime;

use anyhow::{Context, Result};
use p4kx_extraction::{Archive, ExtractionReport, create_default_target, extract, select_entries};

use crate::cli::ExtractArgs;
use crate::exit;

/// Extracts the requested Entries and prints the skipped Entries and a summary to stdout and the
/// failed Entries to stderr.
pub(crate) fn run(args: &ExtractArgs) -> Result<ExitCode> {
    let archive = Archive::open(&args.archive)?;
    let entries = select_entries(&archive, &args.entries)?;
    let target = prepare_target(args)?;
    let report = extract(&archive, &entries, &target, &AtomicBool::new(false));
    if let Err(error) = print_report(&report, &target)
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

fn prepare_target(args: &ExtractArgs) -> Result<PathBuf> {
    if let Some(out) = &args.out {
        fs::create_dir_all(out)
            .with_context(|| format!("cannot create the target folder {}", out.display()))?;
        return Ok(out.clone());
    }
    let parent = std::env::current_dir().context("cannot read the current directory")?;
    create_default_target(&parent, None, None, SystemTime::now())
        .with_context(|| format!("cannot create a target folder in {}", parent.display()))
}

fn print_report(report: &ExtractionReport, target: &Path) -> io::Result<()> {
    let mut err = io::stderr().lock();
    for failure in report.failures() {
        write!(err, "failed {}: {}", failure.path(), failure.error())?;
        let mut source = failure.error().source();
        while let Some(cause) = source {
            write!(err, ": {cause}")?;
            source = cause.source();
        }
        writeln!(err)?;
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
