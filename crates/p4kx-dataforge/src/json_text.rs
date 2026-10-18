use std::fmt;
use std::io::{self, Write};

use crate::xml_text::{Float, FloatText};

const HEX: &[u8; 16] = b"0123456789abcdef";
const INDENT: &[u8; 64] = b"                                                                ";

/// Writes `text` as a JSON string with its quotes.
///
/// `"` and `\` are escaped, line feed, carriage return and tab become `\n`, `\r` and `\t`, the
/// other control characters `\u00XX`; everything else is written as UTF-8.
pub(crate) fn write_string(out: &mut impl Write, text: &str) -> io::Result<()> {
    out.write_all(b"\"")?;
    write_string_content(out, text)?;
    out.write_all(b"\"")
}

fn write_string_content(out: &mut impl Write, text: &str) -> io::Result<()> {
    let bytes = text.as_bytes();
    let mut start = 0;
    for (index, &byte) in bytes.iter().enumerate() {
        let short: &[u8] = match byte {
            b'"' => b"\\\"",
            b'\\' => b"\\\\",
            b'\t' => b"\\t",
            b'\n' => b"\\n",
            b'\r' => b"\\r",
            0..0x20 => b"",
            _ => continue,
        };
        out.write_all(bytes.get(start..index).unwrap_or_default())?;
        if short.is_empty() {
            let high = HEX.get(usize::from(byte >> 4)).copied().unwrap_or(b'0');
            let low = HEX.get(usize::from(byte & 0xf)).copied().unwrap_or(b'0');
            out.write_all(&[b'\\', b'u', b'0', b'0', high, low])?;
        } else {
            out.write_all(short)?;
        }
        start = index + 1;
    }
    out.write_all(bytes.get(start..).unwrap_or_default())
}

/// A [`fmt::Write`] that writes everything as JSON string content into an [`io::Write`].
struct Escaping<'w, W> {
    out: &'w mut W,
    error: Option<io::Error>,
}

impl<W: Write> fmt::Write for Escaping<'_, W> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        write_string_content(self.out, text).map_err(|error| {
            self.error = Some(error);
            fmt::Error
        })
    }
}

/// Writes `value` through its [`fmt::Display`] implementation as a JSON string with its quotes.
pub(crate) fn write_display_string(
    out: &mut impl Write,
    value: impl fmt::Display,
) -> io::Result<()> {
    out.write_all(b"\"")?;
    let mut escaping = Escaping {
        out: &mut *out,
        error: None,
    };
    fmt::write(&mut escaping, format_args!("{value}")).map_err(|_| {
        escaping
            .error
            .take()
            .unwrap_or_else(|| io::Error::other("format failed"))
    })?;
    out.write_all(b"\"")
}

/// Writes a float in the text form of Record XML: a JSON number when finite, otherwise the
/// string `"NaN"`, `"INF"` or `"-INF"`.
pub(crate) fn write_float<F>(out: &mut impl Write, value: F) -> io::Result<()>
where
    F: Float + fmt::Display + fmt::LowerExp,
{
    if value.magnitude().is_finite() {
        write!(out, "{}", FloatText(value))
    } else {
        write!(out, "\"{}\"", FloatText(value))
    }
}

/// Writes a line ending and `depth` levels of two spaces.
pub(crate) fn new_line(out: &mut impl Write, depth: usize) -> io::Result<()> {
    out.write_all(b"\n")?;
    let mut spaces = depth.saturating_mul(2);
    while spaces > 0 {
        let chunk = spaces.min(INDENT.len());
        out.write_all(INDENT.get(..chunk).unwrap_or_default())?;
        spaces -= chunk;
    }
    Ok(())
}
