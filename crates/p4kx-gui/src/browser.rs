// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

mod checks;
mod entry_search;
mod interior;
mod navigation;
mod records;
mod rows;
mod tree;

pub use navigation::HitLine;
pub use rows::{EntryRow, EntryRowKind};

use std::cell::Cell;
use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;

use crate::checks::Checks;
use crate::containers::{Containers, Place};
use crate::record_preview::{IncomingList, Navigation};
use crate::record_search::RecordSearch;
use crate::search::{Matches, Pending};
use crate::{
    AudioPreview, EntryIndex, ExtractionScope, FolderId, ImagePreview, ModelPreview, Preview,
    RecordTree, Sort, SortColumn, SortDirection, SoundbankPreview, TextLanguage,
};

pub use tree::{TreeNode, TreeRow};

/// The browsing state of an opened Archive and its containers.
///
/// It holds the selected and expanded folders, the search, the sorted Entries of the selected
/// folder or of the search, and the checked Entries. The table shows the Entries of one place
/// at a time, the Archive or one container; the search, the checks and the preview belong to
/// that place.
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
    rows: Vec<EntryRow>,
    expanded_textures: HashSet<u32>,
    view_len: usize,
    checks: Checks,
    shown_checked_count: usize,
    preview: Option<Preview>,
    records: Option<RecordTree>,
    record_search: RecordSearch,
    navigation: Navigation,
    incoming: IncomingList,
    record_language: Option<TextLanguage>,
    hit_line: Option<HitLine>,
    hit_serial: u64,
    hit_applied: Cell<u64>,
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
            rows: Vec::new(),
            expanded_textures: HashSet::new(),
            view_len: 0,
            checks,
            shown_checked_count: 0,
            preview: None,
            records,
            record_search: RecordSearch::default(),
            navigation: Navigation::default(),
            incoming: IncomingList::default(),
            record_language: None,
            hit_line: None,
            hit_serial: 0,
            hit_applied: Cell::new(0),
        };
        browser.refresh_tree();
        browser.refresh_shown();
        browser
    }

    /// Returns the index of the place whose Entries are shown: the Archive, or the selected
    /// container once it is open.
    #[must_use]
    pub fn index(&self) -> &EntryIndex {
        self.shown_index()
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

    pub(crate) fn shared_pane(&self) -> Option<&Arc<EntryIndex>> {
        self.pane()
    }

    /// Returns the number of Entries `scope` covers: the checked and the shown Entries of the
    /// shown place, or every Entry of the Archive; none are shown while a search runs.
    #[must_use]
    pub fn scope_len(&self, scope: ExtractionScope) -> usize {
        match scope {
            ExtractionScope::Selected => self.checks.count(),
            ExtractionScope::Shown if self.pending.is_some() => 0,
            ExtractionScope::Shown => self.view_len,
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
            ExtractionScope::Shown => self.shown_with_parts(),
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

    /// Returns the preview of the selected Entry for changing it, or `None` when none is selected.
    pub(crate) const fn preview_mut(&mut self) -> Option<&mut Preview> {
        self.preview.as_mut()
    }

    /// Returns the image preview of the selected Entry when it is the header of a texture.
    pub fn image_mut(&mut self) -> Option<&mut ImagePreview> {
        self.preview.as_mut().and_then(Preview::image_mut)
    }

    /// Returns the model preview of the selected Entry when it is a model file.
    pub fn model_mut(&mut self) -> Option<&mut ModelPreview> {
        self.preview.as_mut().and_then(Preview::model_mut)
    }

    /// Returns the audio preview of the selected Entry when it is a WEM, or that of the selected
    /// WEM when it is a soundbank.
    pub fn audio_mut(&mut self) -> Option<&mut AudioPreview> {
        self.preview.as_mut().and_then(Preview::audio_mut)
    }

    /// Returns the soundbank preview of the selected Entry when it is a soundbank.
    #[must_use]
    pub fn soundbank(&self) -> Option<&SoundbankPreview> {
        self.preview.as_ref().and_then(Preview::soundbank)
    }

    /// Selects the WEM `id` of the selected soundbank and decodes it on a background thread,
    /// unless it is selected already; `wake` is called from that thread once a result is ready.
    pub fn select_sound(&mut self, id: u32, wake: impl Fn() + Send + 'static) {
        if let Some(bank) = self.preview.as_mut().and_then(Preview::soundbank_mut) {
            bank.select(id, wake);
        }
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

    /// Reads again what a missing or wrong Archive key left unread: the DataForge, the texts, the
    /// Records and the preview of the selected Entry.
    pub(crate) fn key_changed(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        self.index.forge().clear();
        self.index.texts().clear();
        self.drop_records();
        if let Some(id) = self.selected_entry() {
            self.preview = None;
            self.select_entry(id, wake);
        }
    }

    /// Takes the result of a finished search, preview and container opening; returns whether
    /// one finished.
    pub fn poll(&mut self) -> bool {
        let previewed = self.preview.as_mut().is_some_and(Preview::poll);
        let loaded = self.records.as_mut().is_some_and(RecordTree::poll);
        let found = self.poll_record_search(loaded);
        let opened = self.poll_containers();
        let read = self.navigation.poll();
        let listed = self.incoming.poll();
        self.poll_search() || previewed || loaded || found || opened || read || listed
    }

    fn folder_range(&self) -> Range<u32> {
        self.pane()
            .map_or(0..0, |index| index.folder(self.folder).entries())
    }

    fn is_in_view(&self, id: u32) -> bool {
        self.matches.as_ref().map_or_else(
            || {
                self.folder_range()
                    .contains(&self.shown_index().texture_header(id))
            },
            |matches| matches.contains(id),
        )
    }

    fn refresh_shown(&mut self) {
        self.refresh_view();
        self.refresh_rows();
    }

    fn refresh_view(&mut self) {
        let Some(index) = self.pane().map(Arc::clone) else {
            self.shown.clear();
            self.view_len = 0;
            self.shown_checked_count = 0;
            return;
        };
        if let Some(matches) = &self.matches {
            self.shown = index.entries_matching(matches.mask(), self.sort);
            self.view_len = matches.mask().iter().filter(|&&matched| matched).count();
            self.shown_checked_count = matches
                .mask()
                .iter()
                .zip(self.checks.marks())
                .filter(|&(&matched, &checked)| matched && checked)
                .count();
        } else {
            self.shown = index.entries_in(self.folder, self.sort);
            let (mut len, mut checked) = (0, 0);
            for &id in &self.shown {
                let parts = index.texture_parts(id);
                len += 1 + parts.len();
                checked += usize::from(self.is_checked(id))
                    + parts.iter().filter(|&&part| self.is_checked(part)).count();
            }
            self.view_len = len;
            self.shown_checked_count = checked;
        }
    }

    pub(crate) fn shown_index(&self) -> &Arc<EntryIndex> {
        self.pane().unwrap_or(&self.index)
    }
}
