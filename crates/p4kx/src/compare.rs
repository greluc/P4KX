use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use p4kx_comparison::{
    Archive, ArchiveError, CompareOptions, Comparison, DirectoryCounts, Filter, Levels, Side,
    compare,
};
use thiserror::Error;

use crate::cli::CompareArgs;
use crate::{exit, json};

const UNKNOWN: &str = "unknown";
const ROOT: &str = "(root)";

/// The JSON report could not be written.
#[derive(Debug, Error)]
#[error("cannot write the report {}", path.display())]
pub(crate) struct ReportNotWritten {
    path: PathBuf,
    #[source]
    source: io::Error,
}

/// Compares both Archives, writes the report when asked, prints the summary or the report as
/// JSON, and returns 0 without differences and 1 with differences.
pub(crate) fn run(args: &CompareArgs, as_json: bool) -> Result<ExitCode> {
    let (old, new) = open_both(&args.old, &args.new)?;
    let filter = args.filter.as_deref().map(Filter::new);
    let options = CompareOptions {
        levels: levels(args),
        filter: filter.as_ref(),
        ..CompareOptions::default()
    };
    let comparison = compare(&old, &new, &options)?;
    if let Some(path) = &args.report {
        write_report(path, &comparison).map_err(|source| ReportNotWritten {
            path: path.clone(),
            source,
        })?;
    }
    if as_json {
        json::print(&comparison)?;
    } else {
        print_summary(&comparison)?;
    }
    Ok(ExitCode::from(if comparison.has_differences() {
        exit::DIFFERENCES
    } else {
        exit::SUCCESS
    }))
}

const fn levels(args: &CompareArgs) -> Levels {
    if args.entries {
        Levels { entries: true }
    } else {
        Levels::ALL
    }
}

fn open_both(old: &Path, new: &Path) -> Result<(Archive, Archive), ArchiveError> {
    std::thread::scope(|scope| {
        let new = scope.spawn(|| Archive::open(new));
        let old = Archive::open(old);
        let new = new
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        Ok((old?, new?))
    })
}

fn write_report(path: &Path, comparison: &Comparison<'_>) -> io::Result<()> {
    let mut out = BufWriter::new(File::create(path)?);
    serde_json::to_writer_pretty(&mut out, comparison)?;
    writeln!(out)?;
    out.into_inner()
        .map_err(io::IntoInnerError::into_error)?
        .sync_all()
}

fn print_summary(comparison: &Comparison<'_>) -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    write_side(&mut out, "Old", comparison.old_side())?;
    write_side(&mut out, "New", comparison.new_side())?;
    write_line(&mut out, "Filter", comparison.filter().unwrap_or("none"))?;
    let counts = comparison.entry_counts();
    write_line(&mut out, "Added", &counts.added.to_string())?;
    write_line(&mut out, "Removed", &counts.removed.to_string())?;
    write_line(&mut out, "Changed", &counts.changed.to_string())?;
    let directories = comparison.directory_counts();
    if !directories.is_empty() {
        writeln!(out)?;
        write_directory(&mut out, ["DIRECTORY", "ADDED", "REMOVED", "CHANGED"])?;
        for DirectoryCounts { directory, counts } in directories {
            let name = if directory.is_empty() {
                ROOT
            } else {
                directory
            };
            write_directory(
                &mut out,
                [
                    name,
                    &counts.added.to_string(),
                    &counts.removed.to_string(),
                    &counts.changed.to_string(),
                ],
            )?;
        }
    }
    out.flush()
}

fn write_side(out: &mut impl Write, label: &str, side: &Side<'_>) -> io::Result<()> {
    write_line(out, label, &json::path(side.archive()))?;
    write_line(
        out,
        &format!("{label} Channel"),
        side.channel().unwrap_or(UNKNOWN),
    )?;
    let build = side.build().map_or_else(
        || UNKNOWN.to_owned(),
        |build| {
            format!(
                "{} ({}, change {})",
                build.version(),
                build.branch(),
                build.change_number()
            )
        },
    );
    write_line(out, &format!("{label} Build"), &build)
}

fn write_line(out: &mut impl Write, label: &str, value: &str) -> io::Result<()> {
    writeln!(out, "{:<16} {value}", format!("{label}:"))
}

fn write_directory(
    out: &mut impl Write,
    [directory, added, removed, changed]: [&str; 4],
) -> io::Result<()> {
    writeln!(out, "{directory:<24} {added:>9} {removed:>9} {changed:>9}")
}
