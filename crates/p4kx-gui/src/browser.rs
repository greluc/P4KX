mod records;

use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;

use p4kx_extraction::Filter;

use crate::search::{self, Matches, Outcome, Pending, Started};
use crate::{
    EntryIndex, ExtractionScope, FolderId, Preview, RecordTree, Sort, SortColumn, SortDirection,
};

/// One visible row of the path tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeRow {
    /// The folder of the row.
    pub folder: FolderId,
    /// How deep the folder sits below the root, which has depth 0.
    pub depth: usize,
}

/// The browsing state of an opened Archive: the selected and expanded folders, the search, the
/// sorted Entries of the selected folder or of the search, and the checked Entries.
#[derive(Debug)]
pub struct Browser {
    index: Arc<EntryIndex>,
    search_text: String,
    matches: Option<Matches>,
    pending: Option<Pending>,
    folder: FolderId,
    expanded: HashSet<FolderId>,
    tree_rows: Vec<TreeRow>,
    sort: Sort,
    shown: Vec<u32>,
    checked: Vec<bool>,
    checked_count: usize,
    shown_checked_count: usize,
    preview: Option<Preview>,
    records: Option<RecordTree>,
}

impl Browser {
    /// Shows every Entry of `index`, sorted by name, with the root expanded.
    #[must_use]
    pub fn new(index: EntryIndex) -> Self {
        let root = index.root();
        let checked = vec![false; index.len()];
        let records = index
            .dataforge_entry()
            .map(|entry| RecordTree::new(entry.path().to_owned()));
        let mut browser = Self {
            index: Arc::new(index),
            search_text: String::new(),
            matches: None,
            pending: None,
            folder: root,
            expanded: HashSet::from([root]),
            tree_rows: Vec::new(),
            sort: Sort::default(),
            shown: Vec::new(),
            checked,
            checked_count: 0,
            shown_checked_count: 0,
            preview: None,
            records,
        };
        browser.refresh_tree();
        browser.refresh_shown();
        browser
    }

    /// Returns the index of the opened Archive.
    #[must_use]
    pub fn index(&self) -> &EntryIndex {
        &self.index
    }

    pub(crate) const fn shared_index(&self) -> &Arc<EntryIndex> {
        &self.index
    }

    /// Returns the number of Entries `scope` covers; none are shown while a search runs.
    #[must_use]
    pub fn scope_len(&self, scope: ExtractionScope) -> usize {
        match scope {
            ExtractionScope::Selected => self.checked_count,
            ExtractionScope::Shown if self.pending.is_some() => 0,
            ExtractionScope::Shown => self.shown.len(),
            ExtractionScope::All => self.index.len(),
        }
    }

    /// Returns the ids of the Entries `scope` covers; none are shown while a search runs.
    #[must_use]
    pub fn scope_entries(&self, scope: ExtractionScope) -> Vec<u32> {
        let ids = (0..self.index.len()).filter_map(|id| u32::try_from(id).ok());
        match scope {
            ExtractionScope::Selected => ids.filter(|&id| self.is_checked(id)).collect(),
            ExtractionScope::Shown if self.pending.is_some() => Vec::new(),
            ExtractionScope::Shown => self.shown.clone(),
            ExtractionScope::All => ids.collect(),
        }
    }

    /// Returns the folder whose Entries are shown.
    #[must_use]
    pub const fn selected_folder(&self) -> FolderId {
        self.folder
    }

    /// Shows the Entries of `folder` and below, ending the search, and expands it.
    pub fn select_folder(&mut self, folder: FolderId) {
        self.folder = folder;
        if let Some(records) = &mut self.records {
            records.clear_selection();
        }
        self.search_text.clear();
        self.pending = None;
        self.matches = None;
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
        let shown = self.is_in_view(id);
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
        let shown = std::mem::take(&mut self.shown);
        for &id in &shown {
            self.set_checked(id, checked);
        }
        self.shown = shown;
    }

    /// Returns the number of checked Entries in the whole Archive.
    #[must_use]
    pub const fn checked_count(&self) -> usize {
        self.checked_count
    }

    /// Returns the number of checked Entries that are not shown.
    #[must_use]
    pub const fn hidden_checked_count(&self) -> usize {
        self.checked_count - self.shown_checked_count
    }

    /// Returns the text of the search field.
    #[must_use]
    pub fn search_text(&self) -> &str {
        &self.search_text
    }

    /// Returns whether the matches of a search are shown rather than the selected folder.
    #[must_use]
    pub const fn shows_search(&self) -> bool {
        self.matches.is_some()
    }

    /// Returns whether a search runs in the background.
    #[must_use]
    pub const fn is_searching(&self) -> bool {
        self.pending.is_some()
    }

    /// Searches the whole Archive for `text` as a Filter on a background thread, replacing a
    /// search still running; `wake` is called from that thread once the result is ready. The
    /// shown Entries change when [`Browser::poll`] takes the result; empty text shows the
    /// selected folder again at once.
    pub fn search(&mut self, text: &str, wake: impl Fn() + Send + 'static) {
        text.clone_into(&mut self.search_text);
        self.pending = None;
        if let Some(records) = self.records.as_mut().filter(|_| !text.is_empty()) {
            records.clear_selection();
        }
        if text.is_empty() {
            self.matches = None;
            self.refresh_shown();
            return;
        }
        match search::start(&self.index, &Filter::new(text), wake) {
            Started::Running(pending) => self.pending = Some(pending),
            Started::Finished(matches) => self.show_matches(matches),
        }
    }

    /// Returns the Entry selected for the preview, or `None` when none is.
    #[must_use]
    pub fn selected_entry(&self) -> Option<u32> {
        self.preview.as_ref().map(Preview::entry)
    }

    /// Returns the preview of the selected Entry, or `None` when none is selected.
    #[must_use]
    pub const fn preview(&self) -> Option<&Preview> {
        self.preview.as_ref()
    }

    /// Selects the Entry `id` and reads its preview on a background thread, unless it is
    /// selected already; `wake` is called from that thread once the data is ready.
    pub fn select_entry(&mut self, id: u32, wake: impl Fn() + Send + 'static) {
        if self.selected_entry() != Some(id) {
            self.preview = Some(Preview::start(&self.index, id, wake));
        }
    }

    /// Takes the result of a finished search and of a finished preview; returns whether one
    /// finished.
    pub fn poll(&mut self) -> bool {
        let previewed = self.preview.as_mut().is_some_and(Preview::poll);
        let loaded = self.records.as_mut().is_some_and(RecordTree::poll);
        self.poll_search() || previewed || loaded
    }

    fn poll_search(&mut self) -> bool {
        let Some(pending) = &self.pending else {
            return false;
        };
        match pending.poll() {
            Outcome::Running => false,
            Outcome::Finished(matches) => {
                self.pending = None;
                self.show_matches(matches);
                true
            }
            Outcome::Lost => {
                self.pending = None;
                true
            }
        }
    }

    fn show_matches(&mut self, matches: Matches) {
        self.matches = Some(matches);
        self.refresh_shown();
    }

    fn folder_range(&self) -> Range<u32> {
        self.index.folder(self.folder).entries()
    }

    fn is_in_view(&self, id: u32) -> bool {
        self.matches.as_ref().map_or_else(
            || self.folder_range().contains(&id),
            |matches| matches.contains(id),
        )
    }

    fn refresh_shown(&mut self) {
        if let Some(matches) = &self.matches {
            self.shown = self.index.entries_matching(matches.mask(), self.sort);
            self.shown_checked_count = matches
                .mask()
                .iter()
                .zip(&self.checked)
                .filter(|&(&matched, &checked)| matched && checked)
                .count();
        } else {
            self.shown = self.index.entries_in(self.folder, self.sort);
            let range = self.folder_range();
            let checked = self.checked.get(range.start as usize..range.end as usize);
            self.shown_checked_count =
                checked.map_or(0, |slots| slots.iter().filter(|&&c| c).count());
        }
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
