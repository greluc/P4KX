use std::fs;
use std::io::{self, BufWriter, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use p4kx_extraction::query::{CompiledQuery, Query, QueryRun, ResultFormat, ResultWriter, Row};
use p4kx_extraction::{Archive, DataForgeEntry, DataForgeError};
use serde::Serialize;
use thiserror::Error;

use crate::cli::{DcbQueryArgs, QueryFormat};
use crate::{exit, json, query_file, query_table};

/// The number of rows built and written at a time.
pub(crate) const PAGE_SIZE: NonZeroUsize = match NonZeroUsize::new(1024) {
    Some(size) => size,
    None => NonZeroUsize::MIN,
};

/// A Query that cannot be parsed or compiled, with its marked position.
#[derive(Debug, Error)]
#[error("{0}")]
pub(crate) struct InvalidQuery(String);

/// `--json` with a format other than JSON and without `--out`, so stdout would not hold JSON.
#[derive(Debug, Error)]
#[error("--json writes JSON to stdout: use --format json, or --out for the {0} result")]
pub(crate) struct JsonNeedsJsonFormat(&'static str);

/// `--out` names the Archive the Query reads.
#[derive(Debug, Error)]
#[error("{} is the Archive the Query reads", .0.display())]
pub(crate) struct OutputIsArchive(PathBuf);

/// The number of rows written and of Records whose row failed.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Counts {
    pub(crate) records: u64,
    pub(crate) failed: u64,
}

#[derive(Serialize)]
struct Summary<'a> {
    archive: String,
    format: &'a str,
    output: String,
    records: u64,
    failed: u64,
}

/// Runs a Query over the Archive's DataForge and writes the result to stdout or `--out`.
pub(crate) fn run(args: &DcbQueryArgs, as_json: bool) -> Result<ExitCode> {
    let format = args.format.unwrap_or(if as_json {
        QueryFormat::Json
    } else {
        QueryFormat::Table
    });
    if as_json && args.out.is_none() && format != QueryFormat::Json {
        return Err(JsonNeedsJsonFormat(name(format)).into());
    }
    if let Some(out) = &args.out
        && is_same_file(out, &args.archive)
    {
        return Err(OutputIsArchive(out.clone()).into());
    }
    let query = Query::parse(&args.query)
        .map_err(|error| InvalidQuery(error.marked(&args.query).to_string()))?;
    let archive = Archive::open(&args.archive)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let compiled = query
        .compile(&forge)
        .map_err(|error| InvalidQuery(error.marked(&args.query).to_string()))?;
    let run = compiled.run();
    let counts = match &args.out {
        None => to_stdout(&compiled, &run, format)?,
        Some(out) => {
            let counts = query_file::write_replacing(out, |file| {
                write_result(&compiled, &run, format, file, false)
            })
            .with_context(|| format!("cannot write {}", out.display()))?;
            print_summary(args, out, format, counts, as_json)?;
            counts
        }
    };
    Ok(ExitCode::from(if counts.failed == 0 {
        exit::SUCCESS
    } else {
        exit::PARTIAL_FAILURE
    }))
}

/// Writes the result to stdout; a closed pipe ends the output early without an error.
fn to_stdout(
    compiled: &CompiledQuery<'_>,
    run: &QueryRun<'_, '_>,
    format: QueryFormat,
) -> io::Result<Counts> {
    let out = BufWriter::new(io::stdout().lock());
    match write_result(compiled, run, format, out, true) {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(Counts::default()),
        written => written,
    }
}

/// Writes the result in `format` to `out`, page by page, and reports failed Records on stderr;
/// with `count_line`, a table ends with the number of Records.
fn write_result(
    compiled: &CompiledQuery<'_>,
    run: &QueryRun<'_, '_>,
    format: QueryFormat,
    out: impl Write,
    count_line: bool,
) -> io::Result<Counts> {
    let format = match format {
        QueryFormat::Table => return query_table::write(compiled.columns(), run, out, count_line),
        QueryFormat::Csv => ResultFormat::Csv,
        QueryFormat::Json => ResultFormat::Json,
    };
    let mut writer = ResultWriter::new(out, format, compiled.columns())?;
    let mut failed = 0_u64;
    for page in run.pages(PAGE_SIZE) {
        for row in &page {
            match row.cells() {
                Ok(cells) => writer.write_row(row.record_path(), cells)?,
                Err(error) => {
                    failed = failed.saturating_add(1);
                    report_failure(row, error)?;
                }
            }
        }
    }
    let records = writer.rows();
    writer.finish()?;
    Ok(Counts { records, failed })
}

/// Reports on stderr that the values of `row`'s Record cannot be read.
pub(crate) fn report_failure(row: &Row<'_>, error: &DataForgeError) -> io::Result<()> {
    writeln!(
        io::stderr().lock(),
        "failed {} ({}): {error}",
        row.record_path(),
        row.record().guid()
    )
}

fn print_summary(
    args: &DcbQueryArgs,
    out: &Path,
    format: QueryFormat,
    counts: Counts,
    as_json: bool,
) -> io::Result<()> {
    if as_json {
        return json::print(&Summary {
            archive: json::path(&args.archive),
            format: name(format),
            output: json::path(out),
            records: counts.records,
            failed: counts.failed,
        });
    }
    let noun = if counts.records == 1 {
        "Record"
    } else {
        "Records"
    };
    let failed = if counts.failed == 0 {
        String::new()
    } else {
        format!(", {} failed,", counts.failed)
    };
    writeln!(
        io::stdout().lock(),
        "{} {noun}{failed} written to {}",
        counts.records,
        out.display()
    )
}

const fn name(format: QueryFormat) -> &'static str {
    match format {
        QueryFormat::Table => "table",
        QueryFormat::Csv => "csv",
        QueryFormat::Json => "json",
    }
}

/// Returns whether `path` names an existing file that is `input`.
fn is_same_file(path: &Path, input: &Path) -> bool {
    match (fs::canonicalize(path), fs::canonicalize(input)) {
        (Ok(path), Ok(input)) => path == input,
        _ => false,
    }
}
