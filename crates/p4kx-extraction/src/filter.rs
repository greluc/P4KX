// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;

use p4kx_archive::{Archive, Entry};
use p4kx_foundation::PathPattern;
use rayon::prelude::*;
use thiserror::Error;

use crate::TextureGroups;

/// The longest Filter text [`Filter::parse`] accepts, in bytes.
pub const MAX_FILTER_LEN: usize = 1024;

/// The most `*` and `?` wildcards [`Filter::parse`] accepts in one Filter.
pub const MAX_FILTER_WILDCARDS: usize = 32;

/// Filter text that [`Filter::parse`] or [`Exclusion::parse`](crate::Exclusion::parse) refuses.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FilterError {
    /// The text is longer than [`MAX_FILTER_LEN`].
    #[error("the Filter is {len} bytes long; at most {MAX_FILTER_LEN} are accepted")]
    TooLong {
        /// The length of the text in bytes.
        len: usize,
    },
    /// The text holds more wildcards than [`MAX_FILTER_WILDCARDS`].
    #[error(
        "the Filter has {count} wildcards (* and ?); at most {MAX_FILTER_WILDCARDS} are accepted"
    )]
    TooManyWildcards {
        /// The number of `*` and `?` in the text.
        count: usize,
    },
    /// An exclusion Filter is empty or only whitespace, which would leave out every Entry or
    /// every Entry with a space in its path.
    #[error("an exclusion Filter is empty or blank, which would leave out nearly every Entry")]
    EmptyExclusion,
    /// More exclusion Filters were given than [`MAX_EXCLUSIONS`](crate::MAX_EXCLUSIONS).
    #[error(
        "{count} exclusion Filters were given; at most {} are accepted",
        crate::MAX_EXCLUSIONS
    )]
    TooManyExclusions {
        /// The number of exclusion Filters given.
        count: usize,
    },
}

/// Text that selects Entries by their path.
///
/// Text with a `*` or `?` is a glob over the full path: `*` matches any run of characters,
/// separators included, and `?` exactly one character. Any other text matches when it occurs
/// anywhere in the path. Both forms ignore ASCII case and treat `/` and `\` alike; every other
/// character, whitespace included, is literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    text: String,
    pattern: PathPattern,
}

impl Filter {
    /// Creates the Filter for `text`.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            pattern: PathPattern::new(text),
        }
    }

    /// Creates the Filter for `text` typed by a person or read from outside: refuses text beyond
    /// [`MAX_FILTER_LEN`] bytes or [`MAX_FILTER_WILDCARDS`] wildcards.
    ///
    /// Matching is a single pass without backtracking over earlier wildcards, so the limits bound
    /// the work per path.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError::TooLong`] or [`FilterError::TooManyWildcards`].
    pub fn parse(text: &str) -> Result<Self, FilterError> {
        Self::check(text)?;
        Ok(Self::new(text))
    }

    /// Checks `text` against the limits of [`Filter::parse`] without building the Filter.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError::TooLong`] or [`FilterError::TooManyWildcards`].
    pub fn check(text: &str) -> Result<(), FilterError> {
        if text.len() > MAX_FILTER_LEN {
            return Err(FilterError::TooLong { len: text.len() });
        }
        let count = text
            .bytes()
            .filter(|byte| matches!(byte, b'*' | b'?'))
            .count();
        if count > MAX_FILTER_WILDCARDS {
            return Err(FilterError::TooManyWildcards { count });
        }
        Ok(())
    }

    /// Returns the text the Filter was created from.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Returns whether the Filter is a glob rather than a substring.
    #[must_use]
    pub const fn is_glob(&self) -> bool {
        self.pattern.is_glob()
    }

    /// Returns whether the Entry path `path` matches.
    #[must_use]
    pub fn matches(&self, path: &str) -> bool {
        self.pattern.matches(path)
    }

    /// Returns the Entries of `archive` whose path matches, in Archive order, and every other
    /// Entry of each texture in `groups` that has one that matches.
    #[must_use]
    pub fn select_with_parts<'a>(
        &self,
        archive: &'a Archive,
        groups: &TextureGroups,
    ) -> Vec<&'a Entry> {
        let entries = archive.entries();
        let mut marked: Vec<bool> = entries
            .par_iter()
            .map(|entry| self.matches(entry.path()))
            .collect();
        groups.complete(&mut marked);
        entries
            .iter()
            .zip(marked)
            .filter_map(|(entry, marked)| marked.then_some(entry))
            .collect()
    }

    /// Returns the Entries of `archive` whose path matches, in Archive order.
    #[must_use]
    pub fn select<'a>(&self, archive: &'a Archive) -> Vec<&'a Entry> {
        archive
            .entries()
            .par_iter()
            .filter(|entry| self.matches(entry.path()))
            .collect()
    }
}

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}
