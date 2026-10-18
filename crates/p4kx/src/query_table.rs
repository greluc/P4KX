use std::fmt::Write as _;
use std::io::{self, Write};

use p4kx_extraction::Value;
use p4kx_extraction::query::{QueryRun, RECORD_PATH_COLUMN};

use crate::query_rows::{Counts, for_each_row};

/// The gap between two columns of the table.
const GAP: &str = "  ";

/// The most values of a list a table cell shows.
const LIST_SHOWN: usize = 8;

/// Writes the result as a table whose columns fit every row: a first pass over the pages
/// measures the columns, a second one writes the rows; with `count_line`, the table ends with
/// the number of Records.
pub(crate) fn write(
    columns: &[String],
    run: &QueryRun<'_, '_>,
    mut out: impl Write,
    count_line: bool,
    counts: &mut Counts,
) -> io::Result<()> {
    let mut header = vec![one_line(RECORD_PATH_COLUMN)];
    header.extend(columns.iter().map(|column| one_line(column)));
    let widths = measure(run, &header);
    line(&mut out, &widths, &header)?;
    let rule: Vec<String> = widths.iter().map(|width| "-".repeat(*width)).collect();
    line(&mut out, &widths, &rule)?;
    for_each_row(run, counts, |row, cells| {
        let mut texts = Vec::with_capacity(cells.len().saturating_add(1));
        texts.push(one_line(row.record_path()));
        texts.extend(
            cells
                .iter()
                .map(|cell| cell.as_ref().map_or_else(String::new, cell_text)),
        );
        line(&mut out, &widths, &texts)
    })?;
    if count_line {
        writeln!(out, "{}", counts.line(""))?;
    }
    out.flush()
}

/// Returns the width of each column: the widest of its heading and its texts in every row.
fn measure(run: &QueryRun<'_, '_>, header: &[String]) -> Vec<usize> {
    let mut widths: Vec<usize> = header.iter().map(|text| text.chars().count()).collect();
    for page in run.pages(p4kx_extraction::QUERY_PAGE_SIZE) {
        for row in &page {
            let Ok(cells) = row.cells() else {
                continue;
            };
            widen(&mut widths, 0, line_width(row.record_path()));
            for (index, cell) in cells.iter().enumerate() {
                if let Some(value) = cell {
                    let width = cell_text(value).chars().count();
                    widen(&mut widths, index.saturating_add(1), width);
                }
            }
        }
    }
    widths
}

fn widen(widths: &mut [usize], index: usize, width: usize) {
    if let Some(current) = widths.get_mut(index) {
        *current = (*current).max(width);
    }
}

/// Returns the text of a cell on one line; a list longer than [`LIST_SHOWN`] values shows its
/// first ones and how many more it holds, e.g. `[1, 2, 3, 4, 5, 6, 7, 8, … 92 more]`.
fn cell_text(value: &Value<'_>) -> String {
    let Value::Array(items) = value else {
        return one_line(&value.to_string());
    };
    let mut text = String::from("[");
    for (index, item) in items.iter().take(LIST_SHOWN).enumerate() {
        if index > 0 {
            text.push_str(", ");
        }
        let _infallible = write!(text, "{item}");
    }
    if items.len() > LIST_SHOWN {
        let _infallible = write!(text, ", … {} more", items.len().saturating_sub(LIST_SHOWN));
    }
    text.push(']');
    one_line(&text)
}

/// Returns `text` on one line: line breaks and tabs escaped, other control characters replaced.
pub(crate) fn one_line(text: &str) -> String {
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
    text.chars().fold(0, |width: usize, c| {
        width.saturating_add(if matches!(c, '\n' | '\r' | '\t') {
            2
        } else {
            1
        })
    })
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
