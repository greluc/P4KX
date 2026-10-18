use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use p4kx_extraction::query::{CompiledQuery, Query, QueryRun, ResultFormat, ResultWriter};
use p4kx_extraction::{Archive, DataForgeEntry};
use serde::Serialize;
use thiserror::Error;

use crate::cli::{DcbQueryArgs, QueryFormat};
use crate::query_rows::{Counts, for_each_row};
use crate::{exit, json, query_file, query_table};

/// A Query that cannot be parsed or compiled, with its marked position.
#[derive(Debug, Error)]
#[error("{0}")]
pub(crate) struct InvalidQuery(String);

/// `--json` with a format other than JSON and without `--out`, so stdout would not hold JSON.
#[derive(Debug, Error)]
#[error("--json writes JSON to stdout: use --format json, or --out for the {} result", .0.name())]
pub(crate) struct JsonNeedsJsonFormat(QueryFormat);

/// `--out` names the Archive the Query reads.
#[derive(Debug, Error)]
#[error("{} is the Archive the Query reads", .0.display())]
pub(crate) struct OutputIsArchive(PathBuf);

#[derive(Serialize)]
struct Summary {
    archive: String,
    format: &'static str,
    output: String,
    records: u64,
    failed: u64,
}

/// A compiled Query with its selected Records, written in one format.
struct Output<'a, 'q, 'f> {
    compiled: &'a CompiledQuery<'f>,
    run: &'a QueryRun<'q, 'f>,
    format: QueryFormat,
}

/// Runs a Query over the Archive's DataForge and writes the result to stdout or `--out`.
pub(crate) fn run(args: &DcbQueryArgs, as_json: bool) -> Result<ExitCode> {
    let format = args.format.unwrap_or(if as_json {
        QueryFormat::Json
    } else {
        QueryFormat::Table
    });
    if as_json && args.out.is_none() && format != QueryFormat::Json {
        return Err(JsonNeedsJsonFormat(format).into());
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
    let output = Output {
        compiled: &compiled,
        run: &run,
        format,
    };
    let mut counts = Counts::default();
    match &args.out {
        None => output.to_stdout(&mut counts)?,
        Some(out) => {
            query_file::write_replacing(out, |file| output.write(file, false, &mut counts))
                .with_context(|| format!("cannot write {}", out.display()))?;
            print_summary(args, out, format, counts, as_json)?;
        }
    }
    Ok(ExitCode::from(if counts.failed == 0 {
        exit::SUCCESS
    } else {
        exit::PARTIAL_FAILURE
    }))
}

impl Output<'_, '_, '_> {
    /// Writes the result to stdout; a closed pipe ends the output early without an error.
    fn to_stdout(&self, counts: &mut Counts) -> io::Result<()> {
        let out = BufWriter::new(io::stdout().lock());
        match self.write(out, true, counts) {
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
            written => written,
        }
    }

    /// Writes the result to `out`, page by page, and reports failed Records on stderr; with
    /// `count_line`, a table ends with the number of Records.
    fn write(&self, out: impl Write, count_line: bool, counts: &mut Counts) -> io::Result<()> {
        let columns = self.compiled.columns();
        let format = match self.format {
            QueryFormat::Table => {
                return query_table::write(columns, self.run, out, count_line, counts);
            }
            QueryFormat::Csv => ResultFormat::Csv,
            QueryFormat::Json => ResultFormat::Json,
        };
        let mut writer = ResultWriter::new(out, format, columns)?;
        for_each_row(self.run, counts, |row, cells| {
            writer.write_row(row.record_path(), cells)
        })?;
        writer.finish().map(drop)
    }
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
            format: format.name(),
            output: json::path(out),
            records: counts.records,
            failed: counts.failed,
        });
    }
    let written = format!(" written to {}", out.display());
    writeln!(io::stdout().lock(), "{}", counts.line(&written))
}

/// Returns whether `path` names an existing file that is `input`.
fn is_same_file(path: &Path, input: &Path) -> bool {
    match (fs::canonicalize(path), fs::canonicalize(input)) {
        (Ok(path), Ok(input)) => path == input,
        _ => false,
    }
}
