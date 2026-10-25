// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use crate::texts::annotation;
use crate::value_json::{write_json_string, write_json_value};
use crate::xml_text::FloatText;
use crate::{Annotations, Field, RecordTree, Resolution, Value};

/// The columns of a flat value export, in order; the text column follows when texts are on.
pub const FLAT_COLUMNS: [&str; 8] = [
    "record_guid",
    "record_name",
    "record_path",
    "field_path",
    "array_indices",
    "kind",
    "value",
    "target_path",
];

/// The name of the column that holds the resolved text of a localisation key.
pub const FLAT_TEXT_COLUMN: &str = "text";

const BOM: &[u8] = b"\xEF\xBB\xBF";

/// How a flat value export is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlatFormat {
    /// CSV with a header row, a UTF-8 byte order mark and CRLF rows; string, localisation and enum
    /// values that a spreadsheet would run as a formula get a leading `'`.
    #[default]
    Csv,
    /// CSV like [`FlatFormat::Csv`] but without byte order mark and without any change to a value.
    CsvRaw,
    /// One compact JSON object per value, ended by a line feed.
    Ndjson,
}

/// Writes Record trees as one row per leaf value, the format `docs/record-flat.md` defines.
///
/// One writer can be reused for many Records; it keeps its scratch space between them. The same
/// tree always gives the same bytes.
#[derive(Debug)]
pub struct FlatValues {
    format: FlatFormat,
    prefix: Vec<u8>,
    path: String,
    indices: Vec<u32>,
}

/// The output of one Record and what it is written with.
struct Out<'w, 'a, W> {
    out: &'w mut W,
    annotations: Annotations<'a>,
}

impl FlatValues {
    /// Creates a writer for `format`.
    #[must_use]
    pub const fn new(format: FlatFormat) -> Self {
        Self {
            format,
            prefix: Vec::new(),
            path: String::new(),
            indices: Vec::new(),
        }
    }

    /// Returns the format the writer writes.
    #[must_use]
    pub const fn format(&self) -> FlatFormat {
        self.format
    }

    /// Writes what starts the file: for [`FlatFormat::Csv`] the byte order mark, for both CSV
    /// formats the header row with the text column when `with_text`; nothing for NDJSON.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of `out`.
    pub fn write_header<W: Write>(&self, with_text: bool, out: &mut W) -> io::Result<()> {
        if self.format == FlatFormat::Ndjson {
            return Ok(());
        }
        if self.format == FlatFormat::Csv {
            out.write_all(BOM)?;
        }
        out.write_all(FLAT_COLUMNS.join(",").as_bytes())?;
        if with_text {
            out.write_all(b",")?;
            out.write_all(FLAT_TEXT_COLUMN.as_bytes())?;
        }
        out.write_all(b"\r\n")
    }

    /// Writes the rows of `tree` to `out`: one per leaf value that is not empty. The texts of
    /// `annotations`, if any, are the text column of localisation keys.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of `out`.
    pub fn write_annotated<W: Write>(
        &mut self,
        tree: &RecordTree<'_>,
        annotations: Annotations<'_>,
        out: &mut W,
    ) -> io::Result<()> {
        self.path.clear();
        self.indices.clear();
        self.prefix.clear();
        if self.format == FlatFormat::Ndjson {
            write_json_prefix(&mut self.prefix, tree)?;
        } else {
            write_csv_prefix(&mut self.prefix, tree)?;
        }
        let mut out = Out { out, annotations };
        self.walk_fields(&mut out, tree.fields())
    }

    fn walk_fields<W: Write>(
        &mut self,
        out: &mut Out<'_, '_, W>,
        fields: &[Field<'_>],
    ) -> io::Result<()> {
        for field in fields {
            let mark = self.path.len();
            if mark > 0 {
                self.path.push('.');
            }
            self.path.push_str(field.name());
            let walked = self.walk_value(out, field.value());
            self.path.truncate(mark);
            walked?;
        }
        Ok(())
    }

    fn walk_value<W: Write>(
        &mut self,
        out: &mut Out<'_, '_, W>,
        value: &Value<'_>,
    ) -> io::Result<()> {
        match value {
            Value::Struct(structure) => self.walk_fields(out, structure.fields()),
            Value::Array(items) => {
                for (index, item) in (0_u32..).zip(items.iter()) {
                    self.indices.push(index);
                    let walked = self.walk_value(out, item);
                    self.indices.pop();
                    walked?;
                }
                Ok(())
            }
            Value::Null | Value::Error(_) => Ok(()),
            Value::String(text) | Value::Locale(text) | Value::Enum(text) if text.is_empty() => {
                Ok(())
            }
            leaf => self.write_row(out, leaf),
        }
    }

    fn write_row<W: Write>(&self, out: &mut Out<'_, '_, W>, value: &Value<'_>) -> io::Result<()> {
        let text = match value {
            Value::Locale(key) => match annotation(&out.annotations, key) {
                Some(Resolution::Text(text)) => Some(text),
                _ => None,
            },
            _ => None,
        };
        let target = match value {
            Value::Reference(reference) => Some(reference.record_path()),
            _ => None,
        };
        if self.format == FlatFormat::Ndjson {
            self.write_json_row(out, value, target, text.as_deref())
        } else {
            self.write_csv_row(out, value, target, text.as_deref())
        }
    }

    fn write_csv_row<W: Write>(
        &self,
        out: &mut Out<'_, '_, W>,
        value: &Value<'_>,
        target: Option<&str>,
        text: Option<&str>,
    ) -> io::Result<()> {
        let neutralise = self.format == FlatFormat::Csv;
        let sink = &mut *out.out;
        sink.write_all(&self.prefix)?;
        write_cell(sink, &self.path, false)?;
        sink.write_all(b",")?;
        write_indices(sink, &self.indices)?;
        write!(sink, ",{},", kind(value))?;
        match value {
            Value::String(text) | Value::Locale(text) | Value::Enum(text) => {
                write_cell(sink, text, neutralise)?;
            }
            Value::Reference(reference) => write!(sink, "{}", reference.guid())?,
            scalar => write_plain(sink, scalar)?,
        }
        sink.write_all(b",")?;
        if let Some(path) = target {
            write_cell(sink, path, false)?;
        }
        if out.annotations.texts().is_some() {
            sink.write_all(b",")?;
            if let Some(text) = text {
                write_cell(sink, text, neutralise)?;
            }
        }
        sink.write_all(b"\r\n")
    }

    fn write_json_row<W: Write>(
        &self,
        out: &mut Out<'_, '_, W>,
        value: &Value<'_>,
        target: Option<&str>,
        text: Option<&str>,
    ) -> io::Result<()> {
        let sink = &mut *out.out;
        sink.write_all(&self.prefix)?;
        sink.write_all(b",\"field_path\":")?;
        write_json_string(sink, &self.path)?;
        sink.write_all(b",\"array_indices\":\"")?;
        write_indices(sink, &self.indices)?;
        write!(sink, "\",\"kind\":\"{}\",\"value\":", kind(value))?;
        match value {
            Value::Reference(reference) => write!(sink, "\"{}\"", reference.guid())?,
            scalar => write_json_value(sink, scalar)?,
        }
        if let Some(path) = target {
            sink.write_all(b",\"target_path\":")?;
            write_json_string(sink, path)?;
        }
        if let Some(text) = text {
            sink.write_all(b",\"text\":")?;
            write_json_string(sink, text)?;
        }
        sink.write_all(b"}\n")
    }
}

fn write_csv_prefix(prefix: &mut Vec<u8>, tree: &RecordTree<'_>) -> io::Result<()> {
    write!(prefix, "{},", tree.guid())?;
    write_cell(prefix, tree.name(), false)?;
    prefix.push(b',');
    write_cell(prefix, tree.record_path(), false)?;
    prefix.push(b',');
    Ok(())
}

fn write_json_prefix(prefix: &mut Vec<u8>, tree: &RecordTree<'_>) -> io::Result<()> {
    write!(
        prefix,
        "{{\"record_guid\":\"{}\",\"record_name\":",
        tree.guid()
    )?;
    write_json_string(prefix, tree.name())?;
    prefix.extend_from_slice(b",\"record_path\":");
    write_json_string(prefix, tree.record_path())
}

/// Returns the `kind` column of a leaf value.
const fn kind(value: &Value<'_>) -> &'static str {
    match value {
        Value::Bool(_) => "bool",
        Value::Int8(_) | Value::Int16(_) | Value::Int32(_) | Value::Int64(_) => "int",
        Value::UInt8(_) | Value::UInt16(_) | Value::UInt32(_) | Value::UInt64(_) => "uint",
        Value::Single(_) => "single",
        Value::Double(_) => "double",
        Value::String(_) => "string",
        Value::Locale(_) => "locale",
        Value::Enum(_) => "enum",
        Value::Guid(_) => "guid",
        _ => "reference",
    }
}

fn write_indices(out: &mut impl Write, indices: &[u32]) -> io::Result<()> {
    for (position, index) in indices.iter().enumerate() {
        if position > 0 {
            out.write_all(b".")?;
        }
        write!(out, "{index}")?;
    }
    Ok(())
}

/// Writes a value that never needs quoting: a number, boolean or GUID.
fn write_plain(out: &mut impl Write, value: &Value<'_>) -> io::Result<()> {
    match value {
        Value::Bool(value) => write!(out, "{value}"),
        Value::Int8(value) => write!(out, "{value}"),
        Value::Int16(value) => write!(out, "{value}"),
        Value::Int32(value) => write!(out, "{value}"),
        Value::Int64(value) => write!(out, "{value}"),
        Value::UInt8(value) => write!(out, "{value}"),
        Value::UInt16(value) => write!(out, "{value}"),
        Value::UInt32(value) => write!(out, "{value}"),
        Value::UInt64(value) => write!(out, "{value}"),
        Value::Single(value) => write!(out, "{}", FloatText(*value)),
        Value::Double(value) => write!(out, "{}", FloatText(*value)),
        Value::Guid(guid) => write!(out, "{guid}"),
        _ => Ok(()),
    }
}

/// Writes `text` as one CSV cell: quoted when it holds a comma, quote, CR or LF; with
/// `neutralise`, a text a spreadsheet would run as a formula starts with `'`.
fn write_cell(out: &mut impl Write, text: &str, neutralise: bool) -> io::Result<()> {
    let prefix: &[u8] = if neutralise && text.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        b"'"
    } else {
        b""
    };
    let quoted = text
        .bytes()
        .any(|byte| matches!(byte, b',' | b'"' | b'\r' | b'\n'));
    if !quoted {
        out.write_all(prefix)?;
        return out.write_all(text.as_bytes());
    }
    out.write_all(b"\"")?;
    out.write_all(prefix)?;
    let bytes = text.as_bytes();
    let mut start = 0;
    for (index, &byte) in bytes.iter().enumerate() {
        if byte == b'"' {
            out.write_all(bytes.get(start..=index).unwrap_or_default())?;
            out.write_all(b"\"")?;
            start = index.saturating_add(1);
        }
    }
    out.write_all(bytes.get(start..).unwrap_or_default())?;
    out.write_all(b"\"")
}
