// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt::{self, Write as _};
use std::io::{self, Write};

use crate::geometry::Rgba;
use crate::scene::Affine;

const NUMBER_CAPACITY: usize = 64;

struct Digits {
    bytes: [u8; NUMBER_CAPACITY],
    len: usize,
}

impl fmt::Write for Digits {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self
            .len
            .checked_add(text.len())
            .filter(|&end| end <= NUMBER_CAPACITY)
            .ok_or(fmt::Error)?;
        self.bytes
            .get_mut(self.len..end)
            .ok_or(fmt::Error)?
            .copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

impl Digits {
    fn trimmed(&self) -> &[u8] {
        let mut text = self.bytes.get(..self.len).unwrap_or_default();
        if text.contains(&b'.') {
            while let Some((&b'0', rest)) = text.split_last() {
                text = rest;
            }
            if let Some((&b'.', rest)) = text.split_last() {
                text = rest;
            }
        }
        if text == b"-0" { b"0" } else { text }
    }
}

/// Writes `value` with at most `decimals` decimals and no trailing zeros; a value that is not
/// finite is written as 0.
pub(crate) fn number(out: &mut impl Write, value: f32, decimals: usize) -> io::Result<()> {
    let mut digits = Digits {
        bytes: [0; NUMBER_CAPACITY],
        len: 0,
    };
    if !value.is_finite() || write!(digits, "{value:.decimals$}").is_err() {
        return out.write_all(b"0");
    }
    out.write_all(digits.trimmed())
}

/// Writes ` name="value"` for a coordinate in twips.
pub(crate) fn coordinate(out: &mut impl Write, name: &str, value: f32) -> io::Result<()> {
    write!(out, " {name}=\"")?;
    number(out, value, 3)?;
    out.write_all(b"\"")
}

/// Writes ` name="matrix(…)"` for an affine transform.
pub(crate) fn matrix(out: &mut impl Write, name: &str, affine: &Affine) -> io::Result<()> {
    write!(out, " {name}=\"matrix(")?;
    for (at, factor) in [affine.a, affine.b, affine.c, affine.d]
        .into_iter()
        .enumerate()
    {
        if at > 0 {
            out.write_all(b" ")?;
        }
        number(out, factor, 8)?;
    }
    for translation in [affine.tx, affine.ty] {
        out.write_all(b" ")?;
        number(out, translation, 4)?;
    }
    out.write_all(b")\"")
}

/// Writes ` name="#rrggbb"`.
pub(crate) fn color(out: &mut impl Write, name: &str, color: Rgba) -> io::Result<()> {
    write!(
        out,
        " {name}=\"#{:02x}{:02x}{:02x}\"",
        color.red, color.green, color.blue
    )
}

/// Writes ` name="alpha"` as a fraction unless the colour is opaque.
pub(crate) fn opacity(out: &mut impl Write, name: &str, value: f32) -> io::Result<()> {
    if value >= 0.999_5 {
        return Ok(());
    }
    write!(out, " {name}=\"")?;
    number(out, value.max(0.0), 4)?;
    out.write_all(b"\"")
}

/// Returns the opacity of an alpha byte.
pub(crate) fn fraction(alpha: u8) -> f32 {
    f32::from(alpha) / 255.0
}

/// Writes `text` as XML character data or attribute value: markup characters become entities and
/// characters XML 1.0 cannot hold become U+FFFD.
pub(crate) fn escaped(out: &mut impl Write, text: &str) -> io::Result<()> {
    let mut buffer = [0_u8; 4];
    for character in text.chars() {
        match character {
            '&' => out.write_all(b"&amp;")?,
            '<' => out.write_all(b"&lt;")?,
            '>' => out.write_all(b"&gt;")?,
            '"' => out.write_all(b"&quot;")?,
            '\'' => out.write_all(b"&apos;")?,
            '\t'
            | '\n'
            | '\r'
            | '\u{20}'..='\u{D7FF}'
            | '\u{E000}'..='\u{FFFD}'
            | '\u{10000}'.. => {
                out.write_all(character.encode_utf8(&mut buffer).as_bytes())?;
            }
            _ => out.write_all("\u{FFFD}".as_bytes())?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{escaped, number};

    fn text(value: f32, decimals: usize) -> String {
        let mut out = Vec::new();
        assert!(number(&mut out, value, decimals).is_ok(), "writes");
        String::from_utf8(out).unwrap_or_default()
    }

    #[test]
    fn numbers_are_short_and_stable() {
        assert_eq!(text(1.0, 3), "1");
        assert_eq!(text(-0.000_1, 3), "0");
        assert_eq!(text(0.050_000_1, 8), "0.0500001");
        assert_eq!(text(12.5, 3), "12.5");
        assert_eq!(text(f32::NAN, 3), "0");
        assert_eq!(
            text(f32::MAX, 3).len(),
            39,
            "the largest float fits the buffer"
        );
    }

    #[test]
    fn markup_and_invalid_characters_are_escaped() {
        let mut out = Vec::new();
        assert!(escaped(&mut out, "a<b>&\"'\u{0}\u{1F}\u{FFFF}ok\u{1F600}").is_ok());
        assert_eq!(
            String::from_utf8(out).unwrap_or_default(),
            "a&lt;b&gt;&amp;&quot;&apos;\u{FFFD}\u{FFFD}\u{FFFD}ok\u{1F600}"
        );
    }
}
