use std::collections::HashSet;
use std::ops::Range;

use crate::{EntryIndex, FolderId, Sort, SortColumn, SortDirection};

/// One visible row of the path tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeRow {
    /// The folder of the row.
    pub folder: FolderId,
    /// How deep the folder sits below the root, which has depth 0.
    pub depth: usize,
}

/// The browsing state of an opened Archive: the selected and expanded folders, the sorted
/// Entries of the selected folder and the checked Entries.
#[derive(Debug)]
pub struct Browser {
    index: EntryIndex,
    folder: FolderId,
    expanded: HashSet<FolderId>,
    tree_rows: Vec<TreeRow>,
    sort: Sort,
    shown: Vec<u32>,
    checked: Vec<bool>,
    checked_count: usize,
    shown_checked_count: usize,
}

impl Browser {
    /// Shows every Entry of `index`, sorted by name, with the root expanded.
    #[must_use]
    pub fn new(index: EntryIndex) -> Self {
        let root = index.root();
        let checked = vec![false; index.len()];
        let mut browser = Self {
            index,
            folder: root,
            expanded: HashSet::from([root]),
            tree_rows: Vec::new(),
            sort: Sort::default(),
            shown: Vec::new(),
            checked,
            checked_count: 0,
            shown_checked_count: 0,
        };
        browser.refresh_tree();
        browser.refresh_shown();
        browser
    }

    /// Returns the index of the opened Archive.
    #[must_use]
    pub const fn index(&self) -> &EntryIndex {
        &self.index
    }

    /// Returns the folder whose Entries are shown.
    #[must_use]
    pub const fn selected_folder(&self) -> FolderId {
        self.folder
    }

    /// Shows the Entries of `folder` and below, and expands it.
    pub fn select_folder(&mut self, folder: FolderId) {
        self.folder = folder;
        if self.expanded.insert(folder) {
            self.refresh_tree();
        }
        self.refresh_shown();
    }

    /// Returns whether `folder` shows its subfolders in the tree.
    #[must_use]
    pub fn is_expanded(&self, folder: FolderId) -> bool {
        self.expanded.contains(&folder)
    }

    /// Expands a collapsed `folder` or collapses an expanded one.
    pub fn toggle_expanded(&mut self, folder: FolderId) {
        if !self.expanded.remove(&folder) {
            self.expanded.insert(folder);
        }
        self.refresh_tree();
    }

    /// Returns the visible rows of the path tree, top to bottom.
    #[must_use]
    pub fn tree_rows(&self) -> &[TreeRow] {
        &self.tree_rows
    }

    /// Returns how the shown Entries are sorted.
    #[must_use]
    pub const fn sort(&self) -> Sort {
        self.sort
    }

    /// Sorts by `column`, reversing the direction when it is already the sort column.
    pub fn sort_by(&mut self, column: SortColumn) {
        self.sort = if self.sort.column == column {
            Sort {
                column,
                direction: self.sort.direction.reversed(),
            }
        } else {
            Sort {
                column,
                direction: SortDirection::Ascending,
            }
        };
        self.refresh_shown();
    }

    /// Returns the ids of the shown Entries in table order.
    #[must_use]
    pub fn shown(&self) -> &[u32] {
        &self.shown
    }

    /// Returns whether the Entry `id` is checked.
    #[must_use]
    pub fn is_checked(&self, id: u32) -> bool {
        self.checked.get(id as usize).copied().unwrap_or(false)
    }

    /// Checks or unchecks the Entry `id`.
    pub fn set_checked(&mut self, id: u32, checked: bool) {
        let shown = self.shown_range().contains(&id);
        let Some(slot) = self.checked.get_mut(id as usize) else {
            return;
        };
        if *slot == checked {
            return;
        }
        *slot = checked;
        if checked {
            self.checked_count += 1;
            self.shown_checked_count += usize::from(shown);
        } else {
            self.checked_count -= 1;
            self.shown_checked_count -= usize::from(shown);
        }
    }

    /// Returns whether at least one Entry is shown and every shown Entry is checked.
    #[must_use]
    pub const fn all_shown_checked(&self) -> bool {
        !self.shown.is_empty() && self.shown_checked_count == self.shown.len()
    }

    /// Checks or unchecks every shown Entry.
    pub fn set_all_shown_checked(&mut self, checked: bool) {
        for id in self.shown_range() {
            self.set_checked(id, checked);
        }
    }

    /// Returns the number of checked Entries in the whole Archive.
    #[must_use]
    pub const fn checked_count(&self) -> usize {
        self.checked_count
    }

    fn shown_range(&self) -> Range<u32> {
        self.index.folder(self.folder).entries()
    }

    fn refresh_shown(&mut self) {
        self.shown = self.index.entries_in(self.folder, self.sort);
        let range = self.shown_range();
        let checked = self.checked.get(range.start as usize..range.end as usize);
        self.shown_checked_count = checked.map_or(0, |slots| slots.iter().filter(|&&c| c).count());
    }

    fn refresh_tree(&mut self) {
        let mut rows = Vec::new();
        let mut pending = vec![TreeRow {
            folder: self.index.root(),
            depth: 0,
        }];
        while let Some(row) = pending.pop() {
            rows.push(row);
            if self.expanded.contains(&row.folder) {
                let children = self.index.folder(row.folder).children();
                pending.extend(children.iter().rev().map(|&folder| TreeRow {
                    folder,
                    depth: row.depth + 1,
                }));
            }
        }
        self.tree_rows = rows;
    }
}
