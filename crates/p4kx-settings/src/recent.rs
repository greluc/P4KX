// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The recently opened Archives, the latest first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecentArchives(Vec<PathBuf>);

impl RecentArchives {
    /// How many Archives are kept.
    pub const LIMIT: usize = 10;

    /// Puts `archive` first, removing an earlier mention and the oldest beyond [`Self::LIMIT`].
    pub fn push(&mut self, archive: PathBuf) {
        self.0.retain(|known| *known != archive);
        self.0.insert(0, archive);
        self.0.truncate(Self::LIMIT);
    }

    /// Returns the Archives, the latest first.
    pub fn iter(&self) -> std::slice::Iter<'_, PathBuf> {
        self.0.iter()
    }

    /// Returns how many Archives are listed.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether no Archive is listed.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> IntoIterator for &'a RecentArchives {
    type Item = &'a PathBuf;
    type IntoIter = std::slice::Iter<'a, PathBuf>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
