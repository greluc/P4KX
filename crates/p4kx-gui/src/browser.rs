mod checks;
mod entry_search;
mod navigation;
mod records;
mod rows;

pub use rows::{EntryRow, EntryRowKind};

use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;

use crate::record_preview::Navigation;
use crate::record_search::RecordSearch;
use crate::search::{Matches, Pending};
use crate::{
    AudioPreview, EntryIndex, ExtractionScope, FolderId, ImagePreview, Preview, RecordTree, Sort,
    SortColumn, SortDirection,
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
    rows: Vec<EntryRow>,
    expanded_textures: HashSet<u32>,
    view_len: usize,
    checked: Vec<bool>,
    checked_count: usize,
    shown_checked_count: usize,
    preview: Option<Preview>,
    records: Option<RecordTree>,
    record_search: RecordSearch,
    navigation: Navigation,
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
            rows: Vec::new(),
            expanded_textures: HashSet::new(),
            view_len: 0,
            checked,
            checked_count: 0,
            shown_checked_count: 0,
            preview: None,
            records,
            record_search: RecordSearch::default(),
            navigation: Navigation::default(),
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
            ExtractionScope::Shown => self.view_len,
            ExtractionScope::All => self.index.len(),
            ExtractionScope::Changes(_) => 0,
        }
    }

    /// Returns the ids of the Entries `scope` covers; none are shown while a search runs.
    #[must_use]
    pub fn scope_entries(&self, scope: ExtractionScope) -> Vec<u32> {
        let ids = (0..self.index.len()).filter_map(|id| u32::try_from(id).ok());
        match scope {
            ExtractionScope::Selected => ids.filter(|&id| self.is_checked(id)).collect(),
            ExtractionScope::Shown if self.pending.is_some() => Vec::new(),
            ExtractionScope::Shown => self.shown_with_parts(),
            ExtractionScope::All => ids.collect(),
            ExtractionScope::Changes(_) => Vec::new(),
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

    /// Returns the ids of the shown Entries in table order; a split texture is one Entry, its
    /// header, with its parts left out.
    #[must_use]
    pub fn shown(&self) -> &[u32] {
        &self.shown
    }

    /// Returns the number of Entries the shown rows stand for, a split texture counting with its
    /// parts.
    #[must_use]
    pub const fn view_len(&self) -> usize {
        self.view_len
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

    /// Returns the preview of the selected Entry for changing it, or `None` when none is selected.
    pub(crate) const fn preview_mut(&mut self) -> Option<&mut Preview> {
        self.preview.as_mut()
    }

    /// Returns the image preview of the selected Entry when it is the header of a texture.
    pub fn image_mut(&mut self) -> Option<&mut ImagePreview> {
        self.preview.as_mut().and_then(Preview::image_mut)
    }

    /// Returns the audio preview of the selected Entry when it is a WEM.
    pub fn audio_mut(&mut self) -> Option<&mut AudioPreview> {
        self.preview.as_mut().and_then(Preview::audio_mut)
    }

    /// Selects the Entry `id` and reads its preview on a background thread, unless it is
    /// selected already; `wake` is called from that thread once the data is ready.
    pub fn select_entry(&mut self, id: u32, wake: impl Fn() + Send + Sync + 'static) {
        if self.selected_entry() != Some(id) {
            self.preview = Some(Preview::start(&self.index, id, wake));
        }
    }

    /// Takes the result of a finished search and of a finished preview; returns whether one
    /// finished.
    pub fn poll(&mut self) -> bool {
        let previewed = self.preview.as_mut().is_some_and(Preview::poll);
        let loaded = self.records.as_mut().is_some_and(RecordTree::poll);
        let found = self.poll_record_search(loaded);
        let read = self.navigation.poll();
        self.poll_search() || previewed || loaded || found || read
    }

    fn folder_range(&self) -> Range<u32> {
        self.index.folder(self.folder).entries()
    }

    fn is_in_view(&self, id: u32) -> bool {
        self.matches.as_ref().map_or_else(
            || self.folder_range().contains(&self.index.texture_header(id)),
            |matches| matches.contains(id),
        )
    }

    fn refresh_shown(&mut self) {
        self.refresh_view();
        self.refresh_rows();
    }

    fn refresh_view(&mut self) {
        if let Some(matches) = &self.matches {
            self.shown = self.index.entries_matching(matches.mask(), self.sort);
            self.view_len = matches.mask().iter().filter(|&&matched| matched).count();
            self.shown_checked_count = matches
                .mask()
                .iter()
                .zip(&self.checked)
                .filter(|&(&matched, &checked)| matched && checked)
                .count();
        } else {
            self.shown = self.index.entries_in(self.folder, self.sort);
            let (mut len, mut checked) = (0, 0);
            for &id in &self.shown {
                let parts = self.index.texture_parts(id);
                len += 1 + parts.len();
                checked += usize::from(self.is_checked(id))
                    + parts.iter().filter(|&&part| self.is_checked(part)).count();
            }
            self.view_len = len;
            self.shown_checked_count = checked;
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
