use std::fmt;
use std::io::{self, Write};

const REPLACEMENT: &[u8] = "\u{fffd}".as_bytes();

/// Returns whether `name` can stand as an element or attribute name without a namespace: an XML
/// 1.0 `NCName` that does not start with `xml` in any case.
pub(crate) fn is_plain_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(is_name_start)
        && chars.all(is_name_char)
        && !name
            .get(..3)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("xml"))
}

const fn is_name_start(c: char) -> bool {
    matches!(c,
        'A'..='Z' | '_' | 'a'..='z'
        | '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}' | '\u{F8}'..='\u{2FF}'
        | '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{1FFF}' | '\u{200C}'..='\u{200D}'
        | '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}' | '\u{3001}'..='\u{D7FF}'
        | '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFD}' | '\u{10000}'..='\u{EFFFF}')
}

const fn is_name_char(c: char) -> bool {
    is_name_start(c)
        || matches!(c,
            '-' | '.' | '0'..='9' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}')
}

/// Writes `text` as the content of a double-quoted attribute value.
///
/// Markup characters and the whitespace that attribute normalisation would change become
/// character references; characters XML 1.0 cannot hold become U+FFFD.
pub(crate) fn write_escaped(out: &mut impl Write, text: &str) -> io::Result<()> {
    let bytes = text.as_bytes();
    let mut start = 0;
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        let (replacement, len): (&[u8], usize) = match byte {
            b'&' => (b"&amp;", 1),
            b'<' => (b"&lt;", 1),
            b'>' => (b"&gt;", 1),
            b'"' => (b"&quot;", 1),
            b'\t' => (b"&#9;", 1),
            b'\n' => (b"&#10;", 1),
            b'\r' => (b"&#13;", 1),
            0..0x20 => (REPLACEMENT, 1),
            0xEF if matches!(bytes.get(index + 1..index + 3), Some([0xBF, 0xBE | 0xBF])) => {
                (REPLACEMENT, 3)
            }
            _ => {
                index += 1;
                continue;
            }
        };
        out.write_all(bytes.get(start..index).unwrap_or_default())?;
        out.write_all(replacement)?;
        index += len;
        start = index;
    }
    out.write_all(bytes.get(start..).unwrap_or_default())
}

/// A [`fmt::Write`] that escapes everything written to it into an [`io::Write`].
pub(crate) struct Escaping<'w, W> {
    pub(crate) out: &'w mut W,
    pub(crate) error: Option<io::Error>,
}

impl<W: Write> fmt::Write for Escaping<'_, W> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        write_escaped(self.out, text).map_err(|error| {
            self.error = Some(error);
            fmt::Error
        })
    }
}

/// Writes `value` escaped through its [`fmt::Display`] implementation.
pub(crate) fn write_display(out: &mut impl Write, value: impl fmt::Display) -> io::Result<()> {
    let mut escaping = Escaping { out, error: None };
    fmt::write(&mut escaping, format_args!("{value}")).map_err(|_| {
        escaping
            .error
            .take()
            .unwrap_or_else(|| io::Error::other("format failed"))
    })
}

/// Writes a float in its one text form: `NaN`, `INF`, `-INF`, otherwise the shortest text that
/// reads back to the same value, with an exponent below 1e-6 and from 1e21 in magnitude.
pub(crate) fn write_float<F>(out: &mut impl Write, value: F) -> io::Result<()>
where
    F: Float + fmt::Display + fmt::LowerExp,
{
    let magnitude = value.magnitude();
    if magnitude.is_nan() {
        out.write_all(b"NaN")
    } else if magnitude.is_infinite() {
        out.write_all(if value.is_negative() { b"-INF" } else { b"INF" })
    } else if magnitude != 0.0 && !(1e-6..1e21).contains(&magnitude) {
        write!(out, "{value:e}")
    } else {
        write!(out, "{value}")
    }
}

/// The float types Record XML writes.
pub(crate) trait Float: Copy {
    /// Returns the absolute value widened to `f64`.
    fn magnitude(self) -> f64;
    /// Returns whether the sign bit is set.
    fn is_negative(self) -> bool;
}

impl Float for f32 {
    fn magnitude(self) -> f64 {
        f64::from(self.abs())
    }

    fn is_negative(self) -> bool {
        self.is_sign_negative()
    }
}

impl Float for f64 {
    fn magnitude(self) -> f64 {
        self.abs()
    }

    fn is_negative(self) -> bool {
        self.is_sign_negative()
    }
}
