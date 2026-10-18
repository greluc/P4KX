use std::fmt;

use schemars::JsonSchema;
use serde::Serialize;

use crate::{CancelToken, Filter};

/// One level a Comparison works on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Entries added, removed or changed, from the central directories.
    Entries,
}

/// The levels a Comparison works on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Levels {
    /// Compares the Entries.
    pub entries: bool,
}

impl Levels {
    /// Every level.
    pub const ALL: Self = Self { entries: true };

    /// Returns the selected levels in report order.
    pub fn selected(self) -> impl Iterator<Item = Level> {
        self.entries.then_some(Level::Entries).into_iter()
    }

    /// Returns whether no level is selected.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        !self.entries
    }
}

impl Default for Levels {
    fn default() -> Self {
        Self::ALL
    }
}

/// How far a Comparison has come.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ComparisonProgress {
    /// Entries of both sides that were joined.
    pub entries_done: usize,
    /// Entries of both sides the Filter selected.
    pub entries_total: usize,
}

/// The settings of one Comparison.
#[derive(Clone, Copy, Default)]
pub struct CompareOptions<'a> {
    /// The levels to compare.
    pub levels: Levels,
    /// Selects the Entries to compare on both sides; `None` compares every Entry.
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
