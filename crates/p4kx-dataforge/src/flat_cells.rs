// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use crate::Value;
use crate::xml_text::FloatText;

/// Writes a value that never needs quoting: a number, boolean or GUID.
pub(crate) fn write_plain(out: &mut impl Write, value: &Value<'_>) -> io::Result<()> {
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
        Value::String(_)
        | Value::Locale(_)
        | Value::Enum(_)
        | Value::Struct(_)
        | Value::Array(_)
        | Value::Reference(_)
        | Value::Null
        | Value::Error(_) => Ok(()),
    }
}

/// Writes `text` as one CSV cell: quoted when it holds a comma, quote, CR or LF; with
/// `neutralise`, a text a spreadsheet would run as a formula starts with `'`.
pub(crate) fn write_cell(out: &mut impl Write, text: &str, neutralise: bool) -> io::Result<()> {
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
