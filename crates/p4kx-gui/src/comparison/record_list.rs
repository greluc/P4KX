// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::sync::Arc;

use p4kx_comparison::{CancelToken, RecordChange, RecordChangeKind, RecordKey};
use rayon::prelude::*;

use crate::SortDirection;
use crate::sort::compare_ignoring_case;

/// A column of the Record list that Records can be sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordChangeColumn {
    /// The change kind: added, removed, moved, changed, unreadable.
    Change,
    /// The record path, ignoring case and separator kind.
    Path,
    /// The type of the Record, ignoring case.
    Type,
}

/// How the Record list is sorted; ties are in path order, reversed when descending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RecordChangeSort {
    /// The column sorted by.
    pub column: RecordChangeColumn,
    /// The direction.
    pub direction: SortDirection,
}

impl Default for RecordChangeSort {
    fn default() -> Self {
        Self {
            column: RecordChangeColumn::Path,
            direction: SortDirection::Ascending,
        }
    }
}

impl RecordChangeSort {
    /// Returns the sort after a click on `column`: reversed when it is the sort column already,
    /// otherwise ascending by it.
    #[must_use]
    pub fn clicked(self, column: RecordChangeColumn) -> Self {
        let direction = if column == self.column {
            self.direction.reversed()
        } else {
            SortDirection::Ascending
        };
        Self { column, direction }
    }
}

/// One added, removed, moved, changed or unreadable Record as the list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordChangeRow {
    key: RecordKey,
    kind: RecordChangeKind,
    old_path: Option<Box<str>>,
    new_path: Option<Box<str>>,
    type_name: Arc<str>,
}

impl RecordChangeRow {
    /// Returns where both Records lie, for comparing them again.
    #[must_use]
    pub const fn key(&self) -> RecordKey {
        self.key
    }

    /// Returns how the Record differs.
    #[must_use]
    pub const fn kind(&self) -> RecordChangeKind {
        self.kind
    }

    /// Returns the record path on the old side, `None` when it has none or it cannot be read.
    #[must_use]
    pub fn old_path(&self) -> Option<&str> {
        self.old_path.as_deref()
    }

    /// Returns the record path on the new side, `None` when it has none or it cannot be read.
    #[must_use]
    pub fn new_path(&self) -> Option<&str> {
        self.new_path.as_deref()
    }

    /// Returns the record path of the new side, else of the old side; empty when neither can be
    /// read.
    #[must_use]
    pub fn path(&self) -> &str {
        self.new_path().or_else(|| self.old_path()).unwrap_or("")
    }

    /// Returns the type of the Record on the new side, else on the old side; empty when neither
    /// can be read.
    #[must_use]
    pub fn type_name(&self) -> &str {
        &self.type_name
    }
}

/// Every Record change of a Comparison in path order, with its orders per column.
#[derive(Debug, Default)]
pub struct RecordChangeList {
    rows: Vec<RecordChangeRow>,
    path: Vec<u32>,
    change: Vec<u32>,
    type_order: Vec<u32>,
}

impl RecordChangeList {
    /// Takes the rows of `changes`, which are in path order, and sorts them by every column, or
    /// returns `None` once `cancel` is set.
    pub(crate) fn build(changes: &[RecordChange<'_>], cancel: &CancelToken) -> Option<Self> {
        let mut types: HashMap<&str, Arc<str>> = HashMap::new();
        let rows: Vec<RecordChangeRow> = changes
            .iter()
            .map(|change| {
                let side = change.new_record().or_else(|| change.old_record());
                let type_name = side.map_or("", |side| side.type_name());
                RecordChangeRow {
                    key: change.key(),
                    kind: change.kind(),
                    old_path: change.old_record().map(|side| side.record_path().into()),
                    new_path: change.new_record().map(|side| side.record_path().into()),
                    type_name: Arc::clone(
                        types.entry(type_name).or_insert_with(|| type_name.into()),
                    ),
                }
            })
            .collect();
        if cancel.is_cancelled() {
            return None;
        }
        let path: Vec<u32> = (0..rows.len())
            .filter_map(|id| u32::try_from(id).ok())
            .collect();
        let row = |id: u32| rows.get(id as usize);
        let (change, type_order) = rayon::join(
            || sorted_by(&path, |a, b| row(*a).map(rank).cmp(&row(*b).map(rank))),
            || {
                sorted_by(&path, |a, b| {
                    let name = |id: u32| row(id).map_or("", RecordChangeRow::type_name);
                    compare_ignoring_case(name(*a), name(*b))
                })
            },
        );
        (!cancel.is_cancelled()).then_some(Self {
            rows,
            path,
            change,
            type_order,
        })
    }

    /// Returns the number of Records.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.rows.len()
    }

    /// Returns whether no Record differs.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Returns the row with id `id`, its position in path order.
    #[must_use]
    pub fn row(&self, id: u32) -> Option<&RecordChangeRow> {
        self.rows.get(id as usize)
    }

    /// Returns every row in path order.
    pub(crate) fn rows(&self) -> &[RecordChangeRow] {
        &self.rows
    }

    /// Returns the ids of the rows in `sort`, keeping those `keep` accepts.
    pub(crate) fn sorted(&self, sort: RecordChangeSort, keep: impl Fn(u32) -> bool) -> Vec<u32> {
        let ids = match sort.column {
            RecordChangeColumn::Path => &self.path,
            RecordChangeColumn::Change => &self.change,
            RecordChangeColumn::Type => &self.type_order,
        }
        .iter()
        .copied();
        match sort.direction {
            SortDirection::Ascending => ids.filter(|&id| keep(id)).collect(),
            SortDirection::Descending => ids.rev().filter(|&id| keep(id)).collect(),
        }
    }
}

/// Returns the position of the kind of `row` in the order Added, Removed, Moved, Changed,
/// Unreadable.
pub(crate) const fn rank(row: &RecordChangeRow) -> u8 {
    kind_rank(row.kind)
}

/// Returns the position of `kind` in the order Added, Removed, Moved, Changed, Unreadable.
pub(crate) const fn kind_rank(kind: RecordChangeKind) -> u8 {
    match kind {
        RecordChangeKind::Added => 0,
        RecordChangeKind::Removed => 1,
        RecordChangeKind::Moved => 2,
        RecordChangeKind::Changed => 3,
        _ => 4,
    }
}

fn sorted_by(ids: &[u32], order: impl Fn(&u32, &u32) -> std::cmp::Ordering + Sync) -> Vec<u32> {
    let mut ids = ids.to_vec();
    ids.par_sort_by(order);
    ids
}
