// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use p4kx_extraction::{CSV_BYTE_ORDER_MARK, Language, TextSet, decode_line_breaks, write_csv_cell};

/// The file format of a localisation table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TableFormat {
    /// One JSON object: `languages`, the language codes in column order, and `entries`, one
    /// object per key with `key` and `texts`, whose members are the language codes.
    Json,
    /// CSV with a UTF-8 byte order mark, CRLF line ends and the header `key` followed by the
    /// language codes; a key or text that a spreadsheet would run as a formula starts with `'`.
    Csv,
    /// As [`Csv`](Self::Csv) with the exact keys and texts: no byte order mark and no `'`.
    CsvRaw,
}

/// Writes every key of `texts` that has a text in one of `languages`, with its text in each of
/// them, sorted by key ignoring ASCII case, and returns the number of keys written.
///
/// Texts have `\n` decoded to a line feed. A language without a text for a key is `null` in JSON
/// and empty in CSV; a text the `global.ini` holds empty is `""` in JSON and empty in CSV. The
/// table is streamed: only the sorted key list is held in memory.
///
/// # Errors
///
/// Returns the [`io::Error`] of the first failed write.
pub fn write_localization_table(
    texts: &TextSet<'_>,
    languages: &[Language],
    format: TableFormat,
    out: &mut impl Write,
) -> io::Result<usize> {
    let rows = texts.keys().into_iter().filter(|key| {
        languages
            .iter()
            .any(|&language| texts.text(key, language).is_some())
    });
    match format {
        TableFormat::Json => write_json(texts, languages, rows, out),
        TableFormat::Csv => write_csv(texts, languages, rows, out, true),
        TableFormat::CsvRaw => write_csv(texts, languages, rows, out, false),
    }
}

fn write_json<'a>(
    texts: &TextSet<'_>,
    languages: &[Language],
    rows: impl Iterator<Item = &'a str>,
    out: &mut impl Write,
) -> io::Result<usize> {
    out.write_all(b"{\"languages\":[")?;
    for (position, language) in languages.iter().enumerate() {
        if position > 0 {
            out.write_all(b",")?;
        }
        serde_json::to_writer(&mut *out, language.code())?;
    }
    out.write_all(b"],\"entries\":[")?;
    let mut count = 0;
    for key in rows {
        out.write_all(if count == 0 { b"\n" } else { b",\n" })?;
        out.write_all(b"{\"key\":")?;
        serde_json::to_writer(&mut *out, key)?;
        out.write_all(b",\"texts\":{")?;
        for (position, &language) in languages.iter().enumerate() {
            if position > 0 {
                out.write_all(b",")?;
            }
            serde_json::to_writer(&mut *out, language.code())?;
            out.write_all(b":")?;
            match texts.text(key, language) {
                Some(text) => serde_json::to_writer(&mut *out, &decode_line_breaks(text))?,
                None => out.write_all(b"null")?,
            }
        }
        out.write_all(b"}}")?;
        count += 1;
    }
    out.write_all(if count == 0 { b"]}\n" } else { b"\n]}\n" })?;
    Ok(count)
}

fn write_csv<'a>(
    texts: &TextSet<'_>,
    languages: &[Language],
    rows: impl Iterator<Item = &'a str>,
    out: &mut impl Write,
    neutralise: bool,
) -> io::Result<usize> {
    if neutralise {
        out.write_all(CSV_BYTE_ORDER_MARK)?;
    }
    out.write_all(b"key")?;
    for language in languages {
        out.write_all(b",")?;
        out.write_all(language.code().as_bytes())?;
    }
    out.write_all(b"\r\n")?;
    let mut count = 0;
    for key in rows {
        write_csv_cell(out, key, neutralise)?;
        for &language in languages {
            out.write_all(b",")?;
            if let Some(text) = texts.text(key, language) {
                write_csv_cell(out, &decode_line_breaks(text), neutralise)?;
            }
        }
        out.write_all(b"\r\n")?;
        count += 1;
    }
    Ok(count)
}
