mod records;
mod tree;

use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;

use p4kx_extraction::Filter;

use crate::checks::Checks;
use crate::containers::{Containers, Place};
use crate::record_search::RecordSearch;
use crate::search::{self, Matches, Outcome, Pending, Started};
use crate::{
    EntryIndex, ExtractionScope, FolderId, ImagePreview, Preview, RecordTree, Sort, SortColumn,
    SortDirection,
};

pub use tree::{TreeNode, TreeRow};

/// The browsing state of an opened Archive and its containers.
///
/// It holds the selected and expanded folders, the search, the sorted Entries of the selected
/// folder or of the search, and the checked Entries. The table shows the Entries of one place at a time, the Archive or one container; the
/// search, the checks and the preview belong to that place.
#[derive(Debug)]
pub struct Browser {
    index: Arc<EntryIndex>,
    containers: Containers,
    place: Place,
    parked: Checks,
    search_text: String,
    matches: Option<Matches>,
    pending: Option<Pending>,
    folder: FolderId,
    expanded: HashSet<TreeNode>,
    tree_rows: Vec<TreeRow>,
    sort: Sort,
    shown: Vec<u32>,
    checks: Checks,
    shown_checked_count: usize,
    preview: Option<Preview>,
    records: Option<RecordTree>,
    record_search: RecordSearch,
}

impl Browser {
    /// Shows every Entry of `index`, sorted by name, with the root expanded.
    #[must_use]
    pub fn new(index: EntryIndex) -> Self {
        let root = index.root();
        let checks = Checks::new(index.len());
        let records = index
            .dataforge_entry()
            .map(|entry| RecordTree::new(entry.path().to_owned()));
        let mut browser = Self {
            index: Arc::new(index),
            containers: Containers::default(),
            place: Place::Archive,
            parked: Checks::default(),
            search_text: String::new(),
            matches: None,
            pending: None,
            folder: root,
            expanded: HashSet::from([TreeNode::from(root)]),
            tree_rows: Vec::new(),
            sort: Sort::default(),
            shown: Vec::new(),
            checks,
            shown_checked_count: 0,
            preview: None,
            records,
            record_search: RecordSearch::default(),
        };
        browser.refresh_tree();
        browser.refresh_shown();
        browser
    }

    /// Returns the index of the place whose Entries are shown: the Archive, or the selected
    /// container once it is open.
    #[must_use]
    pub fn index(&self) -> &EntryIndex {
        self.pane().unwrap_or(&self.index)
    }

    /// Returns the index of the opened Archive itself.
    #[must_use]
    pub fn archive_index(&self) -> &EntryIndex {
        &self.index
    }

    pub(crate) const fn shared_index(&self) -> &Arc<EntryIndex> {
        &self.index
    }

    /// Returns the place whose Entries are shown.
    #[must_use]
    pub const fn place(&self) -> Place {
        self.place
    }

    fn pane(&self) -> Option<&Arc<EntryIndex>> {
        self.containers.index_of(self.place, &self.index)
    }

    /// Returns the number of Entries `scope` covers: the checked and the shown Entries of the
    /// shown place, or every Entry of the Archive; none are shown while a search runs.
    #[must_use]
    pub fn scope_len(&self, scope: ExtractionScope) -> usize {
        match scope {
            ExtractionScope::Selected => self.checks.count(),
            ExtractionScope::Shown if self.pending.is_some() => 0,
            ExtractionScope::Shown => self.shown.len(),
            ExtractionScope::All => self.index.len(),
            ExtractionScope::Changes(_) => 0,
        }
    }

    /// Returns the ids of the Entries `scope` covers: of [`Browser::index`] for the checked and
    /// the shown Entries, of [`Browser::archive_index`] for every Entry; none are shown while a
    /// search runs.
    #[must_use]
    pub fn scope_entries(&self, scope: ExtractionScope) -> Vec<u32> {
        match scope {
            ExtractionScope::Selected => self.checks.ids(),
            ExtractionScope::Shown if self.pending.is_some() => Vec::new(),
            ExtractionScope::Shown => self.shown.clone(),
            ExtractionScope::All => (0..self.index.len())
                .filter_map(|id| u32::try_from(id).ok())
                .collect(),
            ExtractionScope::Changes(_) => Vec::new(),
        }
    }

    /// Returns the index the Entries of `scope` are read from with their ids, or `None` while
    /// the selected container is not open.
    pub(crate) fn scope_source(
        &self,
        scope: ExtractionScope,
    ) -> Option<(Arc<EntryIndex>, Vec<u32>)> {
        let index = match scope {
            ExtractionScope::All => &self.index,
            _ => self.pane()?,
        };
        Some((Arc::clone(index), self.scope_entries(scope)))
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

    /// Returns whether the Entry `id` of the shown place is checked.
    #[must_use]
    pub fn is_checked(&self, id: u32) -> bool {
        self.checks.is_checked(id)
    }

    /// Checks or unchecks the Entry `id` of the shown place.
    pub fn set_checked(&mut self, id: u32, checked: bool) {
        let shown = self.is_in_view(id);
        if self.checks.set(id, checked) && shown {
            if checked {
                self.shown_checked_count += 1;
            } else {
                self.shown_checked_count -= 1;
            }
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

    /// Returns the number of checked Entries of the shown place.
    #[must_use]
    pub const fn checked_count(&self) -> usize {
        self.checks.count()
    }

    /// Returns the number of checked Entries of the shown place that are not shown.
    #[must_use]
    pub const fn hidden_checked_count(&self) -> usize {
        self.checks.count() - self.shown_checked_count
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

    /// Searches the whole shown place for `text` as a Filter on a background thread, replacing
    /// a search still running; `wake` is called from that thread once the result is ready. The
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
        let Some(index) = self.pane().map(Arc::clone) else {
            self.show_matches(Matches::default());
            return;
        };
        match search::start(&index, &Filter::new(text), wake) {
            Started::Running(pending) => self.pending = Some(pending),
            Started::Finished(matches) => self.show_matches(matches),
        }
    }

    /// Returns the Entry of the shown place selected for the preview, or `None` when none is.
    #[must_use]
    pub fn selected_entry(&self) -> Option<u32> {
        self.preview.as_ref().map(Preview::entry)
    }

    /// Returns the preview of the selected Entry, or `None` when none is selected.
    #[must_use]
    pub const fn preview(&self) -> Option<&Preview> {
        self.preview.as_ref()
    }

    /// Returns the image preview of the selected Entry when it is the header of a texture.
    pub fn image_mut(&mut self) -> Option<&mut ImagePreview> {
        self.preview.as_mut().and_then(Preview::image_mut)
    }

    /// Selects the Entry `id` of the shown place and reads its preview on a background thread,
    /// unless it is selected already; `wake` is called from that thread once the data is ready.
    pub fn select_entry(&mut self, id: u32, wake: impl Fn() + Send + Sync + 'static) {
        if self.selected_entry() != Some(id)
            && let Some(index) = self.pane()
        {
            self.preview = Some(Preview::start(index, id, wake));
        }
    }

    /// Takes the result of a finished search, preview and container opening; returns whether
    /// one finished.
    pub fn poll(&mut self) -> bool {
        let previewed = self.preview.as_mut().is_some_and(Preview::poll);
        let loaded = self.records.as_mut().is_some_and(RecordTree::poll);
        let found = self.poll_record_search(loaded);
        let opened = self.poll_containers();
        self.poll_search() || previewed || loaded || found || opened
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
        self.pane()
            .map_or(0..0, |index| index.folder(self.folder).entries())
    }

    fn is_in_view(&self, id: u32) -> bool {
        self.matches.as_ref().map_or_else(
            || self.folder_range().contains(&id),
            |matches| matches.contains(id),
        )
    }

    fn refresh_shown(&mut self) {
        let Some(index) = self.pane().map(Arc::clone) else {
            self.shown.clear();
            self.shown_checked_count = 0;
            return;
        };
        if let Some(matches) = &self.matches {
            self.shown = index.entries_matching(matches.mask(), self.sort);
            self.shown_checked_count = matches
                .mask()
                .iter()
                .zip(self.checks.marks())
                .filter(|&(&matched, &checked)| matched && checked)
                .count();
        } else {
            self.shown = index.entries_in(self.folder, self.sort);
            self.shown_checked_count = self.checks.count_in(self.folder_range());
        }
    }
}
