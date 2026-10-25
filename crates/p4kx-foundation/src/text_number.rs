// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Tolerant reading of numbers in text data: the noise that game files carry inside numbers is
//! dropped, and a value that still is no number is reported instead of read as 0.

use std::borrow::Cow;

/// Returns whether `character` is a Unicode format character (general category Cf, Unicode 16),
/// which is invisible and which game data carries inside numbers, e.g. U+202C.
#[must_use]
pub const fn is_format_character(character: char) -> bool {
    matches!(
        character,
        '\u{00AD}'
            | '\u{0600}'..='\u{0605}'
            | '\u{061C}'
            | '\u{06DD}'
            | '\u{070F}'
            | '\u{0890}'..='\u{0891}'
            | '\u{08E2}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{110BD}'
            | '\u{110CD}'
            | '\u{13430}'..='\u{1343F}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0001}'
            | '\u{E0020}'..='\u{E007F}'
    )
}

const UNIT_SUFFIXES: [&str; 3] = ["d", "f", "kg"];

/// A number read from text, with what the reading had to do to the text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextNumber {
    value: f64,
    cleaned: bool,
    partial: bool,
}

impl TextNumber {
    /// Returns the number read; always finite.
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.value
    }

    /// Returns whether format characters had to be dropped before the number could be read.
    #[must_use]
    pub const fn was_cleaned(&self) -> bool {
        self.cleaned
    }

    /// Returns whether the number is only a prefix of the text: what follows it is neither white
    /// space nor a unit suffix (`d`, `f`, `kg`, ignoring case), e.g. `12,5x` is the partial 12.
    #[must_use]
    pub const fn is_partial(&self) -> bool {
        self.partial
    }
}

/// Reads the longest numeric prefix of `raw` after dropping format characters.
///
/// A trailing unit suffix (`d`, `f`, `kg`) is no part of the number and no partial read; signs and
/// exponents are kept. Returns `None` for text that does not start with a number, such as
/// `v2 1500`, never 0.
#[must_use]
pub fn read_text_number(raw: &str) -> Option<TextNumber> {
    let cleaned = raw.chars().any(is_format_character);
    let text: Cow<'_, str> = if cleaned {
        raw.chars()
            .filter(|character| !is_format_character(*character))
            .collect::<String>()
            .into()
    } else {
        raw.into()
    };
    let text = text.trim();
    let (value, end) = numeric_prefix(text)?;
    let rest = text.get(end..).unwrap_or_default().trim();
    let partial = !(rest.is_empty()
        || UNIT_SUFFIXES
            .iter()
            .any(|unit| rest.eq_ignore_ascii_case(unit)));
    Some(TextNumber {
        value,
        cleaned,
        partial,
    })
}

fn numeric_prefix(text: &str) -> Option<(f64, usize)> {
    let bytes = text.as_bytes();
    let digits = |from: usize| {
        bytes.get(from..).map_or(0, |rest| {
            rest.iter().take_while(|b| b.is_ascii_digit()).count()
        })
    };
    let mut end = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let whole = digits(end);
    end = end.saturating_add(whole);
    let mut fraction = 0;
    if bytes.get(end) == Some(&b'.') {
        fraction = digits(end.saturating_add(1));
        if whole > 0 || fraction > 0 {
            end = end.saturating_add(1).saturating_add(fraction);
        }
    }
    if whole == 0 && fraction == 0 {
        return None;
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(
            bytes.get(end.saturating_add(1)),
            Some(b'+' | b'-')
        ));
        let start = end.saturating_add(1).saturating_add(sign);
        let exponent = digits(start);
        if exponent > 0 {
            end = start.saturating_add(exponent);
        }
    }
    let parsed = text.get(..end)?.parse::<f64>().ok()?;
    parsed.is_finite().then_some((parsed, end))
}
