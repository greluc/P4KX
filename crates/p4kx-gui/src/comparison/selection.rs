// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_comparison::{Archive, ChangeKind, Entry, EntryChange, EntryCounts, OwnedComparison};
use rayon::prelude::*;

use super::ComparisonSide;

/// The added and changed Entries of one side of a Comparison that the Filter selects.
///
/// It shares the Comparison and the Filter result instead of copying the change list; the
/// Entries are resolved on the thread that extracts them.
#[derive(Debug, Clone)]
pub(crate) struct ChangeSelection {
    comparison: Arc<OwnedComparison>,
    matched: Option<Arc<[bool]>>,
    side: ComparisonSide,
    len: usize,
}

impl ChangeSelection {
    pub(crate) const fn new(
        comparison: Arc<OwnedComparison>,
        matched: Option<Arc<[bool]>>,
        side: ComparisonSide,
        counts: EntryCounts,
    ) -> Self {
        Self {
            comparison,
            matched,
            side,
            len: selected_len(side, counts),
        }
    }

    /// Returns the Archive of the chosen side, the one the Entries are read from.
    pub(crate) fn archive(&self) -> &Archive {
        side_archive(&self.comparison, self.side)
    }

    /// Returns the number of selected Entries.
    pub(crate) const fn len(&self) -> usize {
        self.len
    }

    /// Returns the selected Entries of the chosen side in path order.
    pub(crate) fn entries(&self) -> Vec<&Entry> {
        let matched = self.matched.as_deref();
        (0..self.comparison.len())
            .into_par_iter()
            .filter(|&id| matched.is_none_or(|mask| mask.get(id).copied().unwrap_or(false)))
            .filter_map(|id| {
                self.comparison
                    .entry_change(id)
                    .and_then(|change| side_entry(change, self.side))
            })
            .collect()
    }
}

/// Returns the number of Entries of `side` among changes counted as `counts`: the added and the
/// changed ones of the new side, the changed ones of the old side.
pub(crate) const fn selected_len(side: ComparisonSide, counts: EntryCounts) -> usize {
    match side {
        ComparisonSide::Old => counts.changed,
        ComparisonSide::New => counts.added.saturating_add(counts.changed),
    }
}

/// Returns the Archive of `side` of `comparison`.
pub(crate) fn side_archive(comparison: &OwnedComparison, side: ComparisonSide) -> &Archive {
    match side {
        ComparisonSide::Old => comparison.old_archive(),
        ComparisonSide::New => comparison.new_archive(),
    }
}

const fn side_entry(change: EntryChange<'_>, side: ComparisonSide) -> Option<&Entry> {
    match (change.kind(), side) {
        (ChangeKind::Removed, _) => None,
        (_, ComparisonSide::Old) => change.old_entry(),
        (_, ComparisonSide::New) => change.new_entry(),
    }
}
