// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;

use schemars::JsonSchema;
use serde::Serialize;

use crate::{CancelToken, DataForge, Filter};

/// One level a Comparison works on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Level {
    /// Entries added, removed or changed, from the central directories.
    Entries,
    /// Records added, removed, moved or changed, joined by GUID.
    Records,
}

/// The levels a Comparison works on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Levels {
    /// Compares the Entries.
    pub entries: bool,
    /// Compares the Records of both DataForges.
    pub records: bool,
}

impl Levels {
    /// No level.
    pub const NONE: Self = Self {
        entries: false,
        records: false,
    };
    /// Only the Entry level.
    pub const ENTRIES: Self = Self::NONE.with_entries(true);
    /// Only the Record level.
    pub const RECORDS: Self = Self::NONE.with_records(true);
    /// Every level.
    pub const ALL: Self = Self::ENTRIES.with_records(true);

    /// Returns these levels with the Entry level selected or not.
    #[must_use]
    pub const fn with_entries(mut self, selected: bool) -> Self {
        self.entries = selected;
        self
    }

    /// Returns these levels with the Record level selected or not.
    #[must_use]
    pub const fn with_records(mut self, selected: bool) -> Self {
        self.records = selected;
        self
    }

    /// Returns the selected levels in report order.
    pub fn selected(self) -> impl Iterator<Item = Level> {
        [
            self.entries.then_some(Level::Entries),
            self.records.then_some(Level::Records),
        ]
        .into_iter()
        .flatten()
    }

    /// Returns whether no level is selected.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        !self.entries && !self.records
    }
}

impl Default for Levels {
    fn default() -> Self {
        Self::ALL
    }
}

/// The DataForges of the old and the new Archive, which the Record level compares.
#[derive(Debug, Clone, Copy)]
pub struct DataForges<'a> {
    /// The DataForge of the old Archive.
    pub old: &'a DataForge<'a>,
    /// The DataForge of the new Archive.
    pub new: &'a DataForge<'a>,
}

/// How far a Comparison has come.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ComparisonProgress {
    /// Entries of both sides that were joined.
    pub entries_done: usize,
    /// Entries of both sides the Filter selected.
    pub entries_total: usize,
    /// Records both sides have whose fields were compared.
    pub records_done: usize,
    /// Records both sides have, joined by GUID.
    pub records_total: usize,
}

/// The settings of one Comparison.
#[derive(Clone, Copy, Default)]
pub struct CompareOptions<'a> {
    /// The levels to compare.
    pub levels: Levels,
    /// Selects the Entries by path and the Records by the record path of either side; `None`
    /// compares everything.
    pub filter: Option<&'a Filter>,
    /// Stops the Comparison once cancelled.
    pub cancel: Option<&'a CancelToken>,
    /// Called as the Comparison advances.
    pub progress: Option<&'a (dyn Fn(ComparisonProgress) + Sync)>,
}

impl fmt::Debug for CompareOptions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompareOptions")
            .field("levels", &self.levels)
            .field("filter", &self.filter)
            .field("cancel", &self.cancel)
            .field("progress", &self.progress.is_some())
            .finish()
    }
}
