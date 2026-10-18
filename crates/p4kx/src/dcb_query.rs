use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::query::{Query, Row};
use p4kx_extraction::{Archive, DataForgeEntry, Value};
use thiserror::Error;

use crate::cli::DcbQueryArgs;
use crate::exit;

/// The number of rows built and printed at a time.
const PAGE_SIZE: NonZeroUsize = match NonZeroUsize::new(1024) {
    Some(size) => size,
    None => NonZeroUsize::MIN,
};

/// The gap between two columns of the table.
const GAP: &str = "  ";

/// A Query that cannot be parsed or compiled, with its marked position.
#[derive(Debug, Error)]
#[error("{0}")]
pub(crate) struct InvalidQuery(String);

/// `--json` given to `p4kx dcb query`, which prints a table only.
#[derive(Debug, Error)]
#[error("p4kx dcb query prints a table; --json is not supported")]
pub(crate) struct QueryJson;

/// Runs a Query over the Archive's DataForge and prints the result as a table.
pub(crate) fn run(args: &DcbQueryArgs, as_json: bool) -> Result<ExitCode> {
    if as_json {
        return Err(QueryJson.into());
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
    let mut table = Table::new(compiled.columns());
    table.widen_first(run.record_paths().map(line_width));
    let (mut rows, mut failed) = (0_usize, 0_usize);
    let printed = (|| -> io::Result<()> {
        let mut out = io::stdout().lock();
        let mut err = io::stderr().lock();
        for page in run.pages(PAGE_SIZE) {
            let mut lines = Vec::with_capacity(page.len());
            for row in &page {
                match row_texts(row) {
                    Ok(cells) => {
                        rows = rows.saturating_add(1);
                        lines.push(cells);
                    }
                    Err(reason) => {
                        failed = failed.saturating_add(1);
                        writeln!(
                            err,
                            "failed {} ({}): {reason}",
                            row.record_path(),
                            row.record().guid()
                        )?;
                    }
                }
            }
            table.print(&mut out, &lines)?;
        }
        table.print(&mut out, &[])?;
        let noun = if rows == 1 { "Record" } else { "Records" };
        if failed == 0 {
            writeln!(out, "{rows} {noun}")
        } else {
            writeln!(out, "{rows} {noun}, {failed} failed")
        }
    })();
    if let Err(error) = printed
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(error.into());
    }
    Ok(ExitCode::from(if failed == 0 {
        exit::SUCCESS
    } else {
        exit::PARTIAL_FAILURE
    }))
}

/// Returns the texts of a row's columns, the record path first, or why its Record cannot be read.
fn row_texts(row: &Row<'_>) -> Result<Vec<String>, String> {
    let cells = row.cells().map_err(ToString::to_string)?;
    let mut texts = Vec::with_capacity(cells.len() + 1);
    texts.push(one_line(row.record_path()));
    texts.extend(
        cells
            .iter()
            .map(|cell| cell.as_ref().map_or_else(String::new, cell_text)),
    );
    Ok(texts)
}

fn cell_text(value: &Value<'_>) -> String {
    one_line(&value.to_string())
}

/// Returns `text` on one line: line breaks and tabs escaped, other control characters replaced.
fn one_line(text: &str) -> String {
    let mut line = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\n' => line.push_str("\\n"),
            '\r' => line.push_str("\\r"),
            '\t' => line.push_str("\\t"),
            c if c.is_control() => line.push('\u{fffd}'),
            c => line.push(c),
        }
    }
    line
}

/// Returns the number of characters [`one_line`] makes of `text`.
fn line_width(text: &str) -> usize {
    text.chars()
        .map(|c| usize::from(matches!(c, '\n' | '\r' | '\t')) + 1)
        .sum()
}

/// A table printed page by page: the record-path column fits every row, and a value column widens
/// when a later page needs more room.
struct Table {
    header: Option<Vec<String>>,
    widths: Vec<usize>,
}

impl Table {
    fn new(columns: &[String]) -> Self {
        let mut header = vec!["record path".to_owned()];
        header.extend(columns.iter().map(|column| one_line(column)));
        Self {
            widths: header.iter().map(|text| text.chars().count()).collect(),
            header: Some(header),
        }
    }

    /// Widens the first column to the largest of `widths`.
    fn widen_first(&mut self, widths: impl Iterator<Item = usize>) {
        if let Some(width) = self.widths.first_mut() {
            *width = widths.fold(*width, usize::max);
        }
    }

    /// Prints the rows of one page, after the header when it is still due.
    fn print(&mut self, out: &mut impl Write, rows: &[Vec<String>]) -> io::Result<()> {
        for row in rows {
            for (width, text) in self.widths.iter_mut().zip(row) {
                *width = (*width).max(text.chars().count());
            }
        }
        if let Some(header) = self.header.take() {
            self.line(out, &header)?;
            let rule: Vec<String> = self.widths.iter().map(|width| "-".repeat(*width)).collect();
            self.line(out, &rule)?;
        }
        for row in rows {
            self.line(out, row)?;
        }
        Ok(())
    }

    fn line(&self, out: &mut impl Write, cells: &[String]) -> io::Result<()> {
        let mut line = String::new();
        for (index, (width, text)) in self.widths.iter().zip(cells).enumerate() {
            if index > 0 {
                line.push_str(GAP);
            }
            line.push_str(text);
            let padding = width.saturating_sub(text.chars().count());
            line.extend(std::iter::repeat_n(' ', padding));
        }
        writeln!(out, "{}", line.trim_end())
    }
}
