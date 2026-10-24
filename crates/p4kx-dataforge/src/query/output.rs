// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::fmt::Write as _;
use std::io::{self, Write};

use crate::Value;
use crate::value_json::{write_json_string, write_json_value};

/// The heading of the first column of every Query result, the record path.
pub const RECORD_PATH_COLUMN: &str = "record path";

/// A text format of a Query result, defined in `docs/query.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ResultFormat {
    /// CSV after RFC 4180: a header row, CRLF after every row, fields quoted where needed.
    Csv,
    /// One line of JSON: an array with one object per row, keyed by column.
    Json,
}

/// Writes a Query result row by row in a [`ResultFormat`], holding no more than one row.
#[derive(Debug)]
pub struct ResultWriter<W> {
    out: W,
    format: ResultFormat,
    columns: usize,
    keys: Vec<Option<Vec<u8>>>,
    rows: u64,
    text: String,
}

impl<W: Write> ResultWriter<W> {
    /// Starts a result whose columns after the record path have the headings `columns`, and
    /// writes the CSV header row or the opening bracket of the JSON array.
    ///
    /// # Errors
    ///
    /// Returns the [`io::Error`] of writing to `out`.
    pub fn new(out: W, format: ResultFormat, columns: &[String]) -> io::Result<Self> {
        let mut writer = Self {
            out,
            format,
            columns: columns.len(),
            keys: Vec::with_capacity(columns.len()),
            rows: 0,
            text: String::new(),
        };
        match format {
            ResultFormat::Csv => {
                writer.csv_field(RECORD_PATH_COLUMN)?;
                for column in columns {
                    writer.out.write_all(b",")?;
                    writer.csv_field(column)?;
                }
                writer.out.write_all(b"\r\n")?;
            }
            ResultFormat::Json => {
                let mut seen = HashSet::from([RECORD_PATH_COLUMN]);
                for column in columns {
                    let key = if seen.insert(column.as_str()) {
                        let mut key = Vec::new();
                        write_json_string(&mut key, column)?;
                        key.push(b':');
                        Some(key)
                    } else {
                        None
                    };
                    writer.keys.push(key);
                }
                writer.out.write_all(b"[")?;
            }
        }
        Ok(writer)
    }

    /// Writes one row: its record path and one cell per column, `None` for an empty cell.
    ///
    /// A missing cell is written as empty and cells beyond the columns are left out.
    ///
    /// # Errors
    ///
    /// Returns the [`io::Error`] of writing to `out`.
    pub fn write_row(&mut self, record_path: &str, cells: &[Option<Value<'_>>]) -> io::Result<()> {
        match self.format {
            ResultFormat::Csv => self.csv_row(record_path, cells)?,
            ResultFormat::Json => self.json_row(record_path, cells)?,
        }
        self.rows = self.rows.saturating_add(1);
        Ok(())
    }

    /// Returns the number of rows written.
    #[must_use]
    pub const fn rows(&self) -> u64 {
        self.rows
    }

    /// Returns the writer the result goes to.
    #[must_use]
    pub const fn get_ref(&self) -> &W {
        &self.out
    }

    /// Ends the result, flushes it and returns the writer it went to.
    ///
    /// # Errors
    ///
    /// Returns the [`io::Error`] of writing to `out`.
    pub fn finish(mut self) -> io::Result<W> {
        if self.format == ResultFormat::Json {
            self.out.write_all(b"]\n")?;
        }
        self.out.flush()?;
        Ok(self.out)
    }

    fn csv_row(&mut self, record_path: &str, cells: &[Option<Value<'_>>]) -> io::Result<()> {
        self.csv_field(record_path)?;
        for index in 0..self.columns {
            self.out.write_all(b",")?;
            if let Some(Some(value)) = cells.get(index) {
                let mut text = std::mem::take(&mut self.text);
                text.clear();
                let formatted = if matches!(value, Value::Array(_)) {
                    let mut json = Vec::new();
                    write_json_value(&mut json, value).and_then(|()| {
                        text.push_str(
                            std::str::from_utf8(&json)
                                .map_err(|_| io::Error::other("the JSON of a list is not UTF-8"))?,
                        );
                        Ok(())
                    })
                } else {
                    write!(text, "{value}")
                        .map_err(|_| io::Error::other("a value cannot be formatted"))
                };
                let written = formatted.and_then(|()| self.csv_field(&text));
                self.text = text;
                written?;
            }
        }
        self.out.write_all(b"\r\n")
    }

    fn csv_field(&mut self, text: &str) -> io::Result<()> {
        if !text.contains([',', '"', '\r', '\n']) {
            return self.out.write_all(text.as_bytes());
        }
        self.out.write_all(b"\"")?;
        let mut parts = text.split('"');
        if let Some(first) = parts.next() {
            self.out.write_all(first.as_bytes())?;
        }
        for part in parts {
            self.out.write_all(b"\"\"")?;
            self.out.write_all(part.as_bytes())?;
        }
        self.out.write_all(b"\"")
    }

    fn json_row(&mut self, record_path: &str, cells: &[Option<Value<'_>>]) -> io::Result<()> {
        if self.rows > 0 {
            self.out.write_all(b",")?;
        }
        self.out.write_all(b"{")?;
        write_json_string(&mut self.out, RECORD_PATH_COLUMN)?;
        self.out.write_all(b":")?;
        write_json_string(&mut self.out, record_path)?;
        for (index, key) in self.keys.iter().enumerate() {
            let Some(key) = key else {
                continue;
            };
            self.out.write_all(b",")?;
            self.out.write_all(key)?;
            match cells.get(index) {
                Some(Some(value)) => write_json_value(&mut self.out, value)?,
                _ => self.out.write_all(b"null")?,
            }
        }
        self.out.write_all(b"}")
    }
}
