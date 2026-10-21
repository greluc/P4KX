use std::io::{self, BufWriter, Write};

use p4kx_search::{SearchReport, display_text};

const GAP: &str = "  ";

/// Writes one table per source that has hits, text from the Archive made safe to print.
pub(crate) fn print(report: &SearchReport) -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    let written = write(&mut out, report).and_then(|()| out.flush());
    match written {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        written => written,
    }
}

fn write(out: &mut impl Write, report: &SearchReport) -> io::Result<()> {
    let mut started = false;
    let records: Vec<[String; 4]> = report
        .records
        .iter()
        .map(|hit| {
            [
                display_text(&hit.record_path).into_owned(),
                display_text(&hit.name).into_owned(),
                display_text(&hit.field).into_owned(),
                hit.excerpt.text().to_owned(),
            ]
        })
        .collect();
    table(
        out,
        &mut started,
        ["RECORD PATH", "RECORD", "FIELD", "TEXT"],
        &records,
    )?;
    let texts: Vec<[String; 3]> = report
        .localization
        .iter()
        .map(|hit| {
            [
                hit.language.code().to_owned(),
                display_text(&hit.key).into_owned(),
                hit.excerpt.text().to_owned(),
            ]
        })
        .collect();
    table(out, &mut started, ["LANGUAGE", "KEY", "TEXT"], &texts)?;
    let entries: Vec<[String; 2]> = report
        .entries
        .iter()
        .map(|hit| {
            [
                format!("{}:{}", display_text(&hit.path), hit.line),
                hit.excerpt.text().to_owned(),
            ]
        })
        .collect();
    table(out, &mut started, ["ENTRY:LINE", "TEXT"], &entries)
}

/// Writes a header and the rows, with every column but the last as wide as its widest text; a
/// table without rows writes nothing, and a table after another is set off by a blank line.
fn table<const N: usize>(
    out: &mut impl Write,
    started: &mut bool,
    header: [&str; N],
    rows: &[[String; N]],
) -> io::Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    if std::mem::replace(started, true) {
        writeln!(out)?;
    }
    let mut widths = header.map(|title| title.chars().count());
    for row in rows {
        for (width, text) in widths.iter_mut().zip(row) {
            *width = (*width).max(text.chars().count());
        }
    }
    line(out, &widths, header.each_ref().map(|title| *title))?;
    let rule = widths.map(|width| "-".repeat(width));
    line(out, &widths, rule.each_ref().map(String::as_str))?;
    for row in rows {
        line(out, &widths, row.each_ref().map(String::as_str))?;
    }
    Ok(())
}

fn line<const N: usize>(
    out: &mut impl Write,
    widths: &[usize; N],
    cells: [&str; N],
) -> io::Result<()> {
    let mut line = String::new();
    for (index, (width, text)) in widths.iter().zip(cells).enumerate() {
        if index > 0 {
            line.push_str(GAP);
        }
        line.push_str(text);
        if index + 1 < N {
            line.extend(std::iter::repeat_n(
                ' ',
                width.saturating_sub(text.chars().count()),
            ));
        }
    }
    writeln!(out, "{line}")
}
