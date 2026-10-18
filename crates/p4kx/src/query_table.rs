use std::fmt::Write as _;
use std::io::{self, Write};

use p4kx_extraction::Value;
use p4kx_extraction::query::{QueryRun, RECORD_PATH_COLUMN, Row};

use crate::dcb_query::{Counts, PAGE_SIZE, report_failure};

/// The gap between two columns of the table.
const GAP: &str = "  ";

/// Writes the result as a table whose columns fit every row: a first pass over the pages
/// measures the columns, a second one writes the rows; with `count_line`, the table ends with
/// the number of Records.
pub(crate) fn write(
    columns: &[String],
    run: &QueryRun<'_, '_>,
    mut out: impl Write,
    count_line: bool,
) -> io::Result<Counts> {
    let mut header = vec![one_line(RECORD_PATH_COLUMN)];
    header.extend(columns.iter().map(|column| one_line(column)));
    let mut widths: Vec<usize> = header.iter().map(|text| text.chars().count()).collect();
    let mut text = String::new();
    for page in run.pages(PAGE_SIZE) {
        for row in &page {
            let Ok(cells) = row.cells() else {
                continue;
            };
            widen(&mut widths, 0, line_width(row.record_path()));
            for (index, cell) in cells.iter().enumerate() {
                if let Some(value) = cell {
                    text.clear();
                    let _infallible = write!(text, "{value}");
                    widen(&mut widths, index.saturating_add(1), line_width(&text));
                }
            }
        }
    }
    line(&mut out, &widths, &header)?;
    let rule: Vec<String> = widths.iter().map(|width| "-".repeat(*width)).collect();
    line(&mut out, &widths, &rule)?;
    let mut counts = Counts::default();
    for page in run.pages(PAGE_SIZE) {
        for row in &page {
            match row_texts(row) {
                Ok(cells) => {
                    counts.records = counts.records.saturating_add(1);
                    line(&mut out, &widths, &cells)?;
                }
                Err(error) => {
                    counts.failed = counts.failed.saturating_add(1);
                    report_failure(row, error)?;
                }
            }
        }
    }
    if count_line {
        let noun = if counts.records == 1 {
            "Record"
        } else {
            "Records"
        };
        if counts.failed == 0 {
            writeln!(out, "{} {noun}", counts.records)?;
        } else {
            writeln!(out, "{} {noun}, {} failed", counts.records, counts.failed)?;
        }
    }
    out.flush()?;
    Ok(counts)
}

fn widen(widths: &mut [usize], index: usize, width: usize) {
    if let Some(current) = widths.get_mut(index) {
        *current = (*current).max(width);
    }
}

/// Returns the texts of a row's columns, the record path first, or why its Record cannot be read.
fn row_texts<'r>(row: &'r Row<'_>) -> Result<Vec<String>, &'r p4kx_extraction::DataForgeError> {
    let cells = row.cells()?;
    let mut texts = Vec::with_capacity(cells.len().saturating_add(1));
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

fn line(out: &mut impl Write, widths: &[usize], cells: &[String]) -> io::Result<()> {
    let mut line = String::new();
    for (index, (width, text)) in widths.iter().zip(cells).enumerate() {
        if index > 0 {
            line.push_str(GAP);
        }
        line.push_str(text);
        let padding = width.saturating_sub(text.chars().count());
        line.extend(std::iter::repeat_n(' ', padding));
    }
    writeln!(out, "{}", line.trim_end())
}
