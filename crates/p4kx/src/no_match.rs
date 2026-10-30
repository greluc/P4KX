// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use thiserror::Error;

/// A Filter that matches no Entry of the Archive.
#[derive(Debug, Error)]
#[error("no Entry matches {0}")]
pub(crate) struct NoMatch(String);

impl NoMatch {
    /// Creates the error for the Filter `text`.
    pub(crate) fn new(text: &str) -> Self {
        Self(text.to_owned())
    }

    /// Returns what to try instead, after the way the Filter text is matched.
    pub(crate) fn hint(&self) -> &'static str {
        let text = &self.0;
        if text.trim() != text {
            "the Filter has leading or trailing whitespace, which is matched literally"
        } else if text.contains(['*', '?']) && !text.starts_with('*') {
            "a glob must match the whole path, which starts at its first folder; start the \
             Filter with * to match anywhere"
        } else if text.contains(['*', '?']) {
            "a glob must match the whole path; end the Filter with * to match what follows, or \
             drop the * and ? for text anywhere in the path"
        } else {
            "a Filter without * or ? matches text anywhere in the path, ignoring case and \
             treating / and \\ alike; try less text, or see the paths with `p4kx search`"
        }
    }
}

/// Returns the line that tells what to try when `error` is a Filter that matched no Entry.
pub(crate) fn no_match_hint(error: &anyhow::Error) -> Option<&'static str> {
    error.downcast_ref::<NoMatch>().map(NoMatch::hint)
}
