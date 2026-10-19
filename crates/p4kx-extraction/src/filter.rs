use std::fmt;

use p4kx_archive::{Archive, Entry};
use p4kx_foundation::PathPattern;
use rayon::prelude::*;

use crate::TextureGroups;

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
            .iter()
            .filter(|entry| self.matches(entry.path()))
            .collect()
    }
}

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}
