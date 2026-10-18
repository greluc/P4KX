use std::io::{self, Write};

const REPLACEMENT: &[u8] = "\u{FFFD}".as_bytes();

/// Where escaped text goes in the XML.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Context {
    /// A double-quoted attribute value: whitespace other than space is kept as a reference.
    Attribute,
    /// Element content.
    Text,
}

/// Writes `text` with markup escaped, CR as a reference and characters XML 1.0 forbids replaced
/// by U+FFFD.
pub(crate) fn write_escaped(out: &mut impl Write, text: &str, context: Context) -> io::Result<()> {
    let bytes = text.as_bytes();
    let mut start = 0;
    for (index, &byte) in bytes.iter().enumerate() {
        let replacement: &[u8] = match byte {
            b'&' => b"&amp;",
            b'<' => b"&lt;",
            b'>' => b"&gt;",
            b'"' if context == Context::Attribute => b"&quot;",
            b'\r' => b"&#13;",
            b'\n' if context == Context::Attribute => b"&#10;",
            b'\t' if context == Context::Attribute => b"&#9;",
            b'\n' | b'\t' => continue,
            0..0x20 => REPLACEMENT,
            0xEF if is_noncharacter(bytes.get(index + 1..index + 3)) => {
                out.write_all(bytes.get(start..index).unwrap_or_default())?;
                out.write_all(REPLACEMENT)?;
                start = index + 3;
                continue;
            }
            _ => continue,
        };
        out.write_all(bytes.get(start..index).unwrap_or_default())?;
        out.write_all(replacement)?;
        start = index + 1;
    }
    out.write_all(bytes.get(start..).unwrap_or_default())
}

const fn is_noncharacter(continuation: Option<&[u8]>) -> bool {
    matches!(continuation, Some([0xBF, 0xBE | 0xBF]))
}

/// Writes `name` with every character an XML name cannot hold replaced by `_`.
pub(crate) fn write_name(out: &mut impl Write, name: &str) -> io::Result<()> {
    if name.is_empty() {
        return out.write_all(b"_");
    }
    let mut start = 0;
    for (index, character) in name.char_indices() {
        let valid = character.is_alphabetic()
            || character == '_'
            || character == ':'
            || (index > 0 && (character.is_alphanumeric() || matches!(character, '-' | '.')));
        if !valid {
            out.write_all(name.get(start..index).unwrap_or_default().as_bytes())?;
            out.write_all(b"_")?;
            start = index + character.len_utf8();
        }
    }
    out.write_all(name.get(start..).unwrap_or_default().as_bytes())
}
