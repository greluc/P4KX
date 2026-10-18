use std::fmt;
use std::io::{self, Write};

use crate::Value;
use crate::xml_text::{Float, FloatText};

/// Writes `value` as a typed JSON value: a boolean, a number, a string, or `{"error": reason}`
/// for a value that cannot be read.
///
/// Integers are exact decimal numbers; floats are numbers in their Record XML text, `NaN`,
/// `INF` and `-INF` are strings; strings, localisation keys, GUIDs and enum options are strings.
/// A null pointer is `null`; a struct, an array or a Reference is the string of its summary.
pub(crate) fn write_json_value(out: &mut impl Write, value: &Value<'_>) -> io::Result<()> {
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
        Value::Single(value) => write_float(out, *value, value.is_finite()),
        Value::Double(value) => write_float(out, *value, value.is_finite()),
        Value::String(text) | Value::Locale(text) | Value::Enum(text) => {
            write_json_string(out, text)
        }
        Value::Guid(guid) => write!(out, "\"{guid}\""),
        Value::Null => out.write_all(b"null"),
        Value::Struct(_) | Value::Array(_) | Value::Reference(_) => {
            write_json_string(out, &value.to_string())
        }
        Value::Error(error) => {
            out.write_all(b"{\"error\":")?;
            write_json_string(out, &error.to_string())?;
            out.write_all(b"}")
        }
    }
}

fn write_float<F>(out: &mut impl Write, value: F, finite: bool) -> io::Result<()>
where
    F: Float + fmt::Display + fmt::LowerExp,
{
    if finite {
        write!(out, "{}", FloatText(value))
    } else {
        write!(out, "\"{}\"", FloatText(value))
    }
}

/// Writes `text` as a JSON string: `"` and `\` escaped, line feed, carriage return and tab as
/// `\n`, `\r` and `\t`, other control characters below U+0020 as `\u00XX`.
pub(crate) fn write_json_string(out: &mut impl Write, text: &str) -> io::Result<()> {
    out.write_all(b"\"")?;
    let bytes = text.as_bytes();
    let mut start = 0;
    for (index, &byte) in bytes.iter().enumerate() {
        let short: &[u8] = match byte {
            b'"' => b"\\\"",
            b'\\' => b"\\\\",
            b'\n' => b"\\n",
            b'\r' => b"\\r",
            b'\t' => b"\\t",
            0..0x20 => b"",
            _ => continue,
        };
        out.write_all(bytes.get(start..index).unwrap_or_default())?;
        if short.is_empty() {
            write!(out, "\\u{byte:04x}")?;
        } else {
            out.write_all(short)?;
        }
        start = index.saturating_add(1);
    }
    out.write_all(bytes.get(start..).unwrap_or_default())?;
    out.write_all(b"\"")
}
