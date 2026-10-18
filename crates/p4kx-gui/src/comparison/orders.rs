use p4kx_comparison::{ChangeKind, Entry, OwnedComparison};
use rayon::prelude::*;

use crate::SortDirection;

/// A column of the change list that changes can be sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeColumn {
    /// The change kind: added, removed, changed.
    Change,
    /// The Entry path, ignoring case and separator kind.
    Path,
    /// The size on the old side; added Entries have none.
    OldSize,
    /// The size on the new side; removed Entries have none.
    NewSize,
}

/// How the change list is sorted; ties are in path order, reversed when descending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChangeSort {
    /// The column sorted by.
    pub column: ChangeColumn,
    /// The direction.
    pub direction: SortDirection,
}

impl Default for ChangeSort {
    fn default() -> Self {
        Self {
            column: ChangeColumn::Path,
            direction: SortDirection::Ascending,
        }
    }
}

impl ChangeSort {
    /// Returns the sort after a click on `column`: reversed when it is the sort column already,
    /// otherwise ascending by it.
    #[must_use]
    pub fn clicked(self, column: ChangeColumn) -> Self {
        let direction = if column == self.column {
            self.direction.reversed()
        } else {
            SortDirection::Ascending
        };
        Self { column, direction }
    }
}

/// Every change id once per column, in ascending order of that column.
#[derive(Debug, Default)]
pub(crate) struct ChangeOrders {
    path: Vec<u32>,
    change: Vec<u32>,
    old_size: Vec<u32>,
    new_size: Vec<u32>,
}

impl ChangeOrders {
    /// Sorts the ids of the changes of `comparison`, which are in path order, by every column.
    pub(crate) fn build(comparison: &OwnedComparison) -> Self {
        let path: Vec<u32> = (0..comparison.len())
            .filter_map(|id| u32::try_from(id).ok())
            .collect();
        let change = |id: u32| comparison.entry_change(id as usize);
        let (kind, (old_size, new_size)) = rayon::join(
            || sorted_by_key(&path, |id| change(id).map(|c| rank(c.kind()))),
            || {
                rayon::join(
                    || {
                        sorted_by_key(&path, |id| {
                            change(id).and_then(|c| c.old_entry().map(Entry::uncompressed_size))
                        })
                    },
                    || {
                        sorted_by_key(&path, |id| {
                            change(id).and_then(|c| c.new_entry().map(Entry::uncompressed_size))
                        })
                    },
                )
            },
        );
        Self {
            path,
            change: kind,
            old_size,
            new_size,
        }
    }

    /// Returns the ids in ascending order of `column`.
    pub(crate) fn ascending(&self, column: ChangeColumn) -> &[u32] {
        match column {
            ChangeColumn::Change => &self.change,
            ChangeColumn::Path => &self.path,
            ChangeColumn::OldSize => &self.old_size,
            ChangeColumn::NewSize => &self.new_size,
        }
    }
}

/// Returns the position of `kind` in the order Added, Removed, Changed.
pub(crate) const fn rank(kind: ChangeKind) -> u8 {
    match kind {
        ChangeKind::Added => 0,
        ChangeKind::Removed => 1,
        ChangeKind::Changed => 2,
    }
}

fn sorted_by_key<K: Ord + Send + Sync>(ids: &[u32], key: impl Fn(u32) -> K + Sync) -> Vec<u32> {
    let mut ids = ids.to_vec();
    ids.par_sort_by_cached_key(|&id| key(id));
    ids
}
