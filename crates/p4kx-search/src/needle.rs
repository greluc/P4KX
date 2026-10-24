// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::ops::Range;

use thiserror::Error;

/// The text to search for is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("the text to search for is empty")]
pub struct EmptyNeedle;

/// A text searched for as a substring, ignoring case.
///
/// Case is folded with Unicode lower-casing, character by character, so `GLADI` finds
/// `AEGS_Gladius` and `STRASSE` does not find `Straße`. Matching allocates nothing; text that is
/// ASCII on both sides takes a byte loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Needle {
    folded: Vec<char>,
    ascii: Option<Box<[u8]>>,
}

impl Needle {
    /// Creates the needle for `text`.
    ///
    /// # Errors
    ///
    /// Returns [`EmptyNeedle`] when `text` is empty.
    pub fn new(text: &str) -> Result<Self, EmptyNeedle> {
        if text.is_empty() {
            return Err(EmptyNeedle);
        }
        let ascii = text
            .is_ascii()
            .then(|| text.bytes().map(|byte| byte.to_ascii_lowercase()).collect());
        Ok(Self {
            folded: text.chars().flat_map(char::to_lowercase).collect(),
            ascii,
        })
    }

    /// Returns the byte range of the first occurrence in `haystack`, or `None`.
    #[must_use]
    pub fn find(&self, haystack: &str) -> Option<Range<usize>> {
        match &self.ascii {
            Some(needle) if haystack.is_ascii() => find_ascii(haystack.as_bytes(), needle),
            _ => self.find_folded(haystack),
        }
    }

    /// Returns whether `haystack` holds the text.
    #[must_use]
    pub fn is_in(&self, haystack: &str) -> bool {
        self.find(haystack).is_some()
    }

    fn find_folded(&self, haystack: &str) -> Option<Range<usize>> {
        let first = *self.folded.first()?;
        haystack.char_indices().find_map(|(start, c)| {
            if c.to_lowercase().next() != Some(first) {
                return None;
            }
            self.ends_at(haystack, start).map(|end| start..end)
        })
    }

    fn ends_at(&self, haystack: &str, start: usize) -> Option<usize> {
        let mut wanted = self.folded.iter();
        let mut end = start;
        for c in haystack.get(start..)?.chars() {
            for lower in c.to_lowercase() {
                if wanted.next() != Some(&lower) {
                    return None;
                }
            }
            end += c.len_utf8();
            if wanted.as_slice().is_empty() {
                return Some(end);
            }
        }
        None
    }
}

fn find_ascii(haystack: &[u8], needle: &[u8]) -> Option<Range<usize>> {
    let (&first, rest) = needle.split_first()?;
    let last_start = haystack.len().checked_sub(needle.len())?;
    let mut from = 0;
    while from <= last_start {
        let window = haystack.get(from..=last_start)?;
        let start = from
            + window
                .iter()
                .position(|b| b.to_ascii_lowercase() == first)?;
        let tail = haystack.get(start + 1..start + needle.len())?;
        if tail
            .iter()
            .zip(rest)
            .all(|(byte, wanted)| byte.to_ascii_lowercase() == *wanted)
        {
            return Some(start..start + needle.len());
        }
        from = start + 1;
    }
    None
}
