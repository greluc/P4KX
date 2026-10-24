// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use crate::record_search::Wake;
use crate::{Browser, RecordId, RecordNodeId, RecordNodeKind, RecordTree, RecordTreeStatus};

impl Browser {
    /// Returns the DataForge node of the path tree, or `None` when the Archive holds neither
    /// `Data\Game2.dcb` nor `Data\Game.dcb`.
    #[must_use]
    pub const fn records(&self) -> Option<&RecordTree> {
        self.records.as_ref()
    }

    /// Expands or collapses the Record tree node `id`; the first time DataForge is needed it is
    /// loaded on a background thread, calling `wake` from it once it is ready.
    pub fn toggle_record_node(&mut self, id: RecordNodeId, wake: impl Fn() + Send + 'static) {
        if let Some(records) = &mut self.records {
            records.toggle(id, &self.index, wake);
        }
    }

    /// Selects the Record tree node `id` in place of a folder, loading DataForge like
    /// [`Browser::toggle_record_node`].
    pub fn select_record_node(&mut self, id: RecordNodeId, wake: impl Fn() + Send + 'static) {
        if let Some(records) = &mut self.records {
            records.select(id, &self.index, wake);
            if let RecordNodeKind::Record(record) = records.node(id).kind() {
                self.navigation.visit(record);
            }
        }
    }

    /// Loads DataForge on a background thread unless it is loaded or loading already, calling
    /// `wake` from it once it is ready.
    pub fn load_records(&mut self, wake: impl Fn() + Send + 'static) {
        if let Some(records) = &mut self.records {
            records.load(&self.index, wake);
        }
    }

    /// Returns whether the list panel shows the Record search: a node of the Record tree is
    /// selected instead of a folder of the Archive.
    #[must_use]
    pub fn shows_records(&self) -> bool {
        self.records().and_then(RecordTree::selected).is_some()
    }

    /// Returns the text the Records were last searched for.
    #[must_use]
    pub fn record_search_text(&self) -> &str {
        self.record_search.text()
    }

    /// Returns the Records the last finished Record search matched, in record path order, or
    /// `None` when there is none.
    #[must_use]
    pub fn record_matches(&self) -> Option<&[RecordId]> {
        self.record_search.matches()
    }

    /// Returns whether a Record search runs in the background or waits for DataForge.
    #[must_use]
    pub const fn is_searching_records(&self) -> bool {
        self.record_search.is_searching()
    }

    /// Returns a number that changes whenever the Record matches change, for caches.
    pub(crate) const fn record_search_generation(&self) -> u64 {
        self.record_search.generation()
    }

    /// Searches the Records for `text` by record path and type name with the rules of a Filter
    /// on a background thread, replacing a search still running; DataForge is loaded first when
    /// needed. `wake` is called from those threads once a result is ready, which
    /// [`Browser::poll`] takes. Empty text ends the search.
    pub fn search_records(&mut self, text: &str, wake: impl Fn() + Send + Sync + 'static) {
        let Some(records) = &mut self.records else {
            return;
        };
        let wake: Wake = Arc::new(wake);
        if !text.is_empty() {
            let loaded = Arc::clone(&wake);
            records.load(&self.index, move || loaded());
        }
        let index = records.shared_index();
        let unavailable = index.is_none() && !matches!(records.status(), RecordTreeStatus::Loading);
        self.record_search.search(text, index, wake);
        if unavailable {
            self.record_search.stop();
        }
    }

    /// Selects the Record `id` in the Record tree, expanding the folders above it and scrolling
    /// the tree to it, as the next step of the way taken.
    pub fn pick_record(&mut self, id: RecordId) {
        if let Some(records) = &mut self.records
            && records.reveal(id)
        {
            self.navigation.visit(id);
        }
    }

    /// Takes the row of the path tree the tree was asked to scroll to, counted from the root
    /// folder.
    pub(crate) fn take_tree_scroll(&mut self) -> Option<usize> {
        let row = self.records.as_mut()?.take_scroll_request()?;
        self.tree_rows().len().checked_add(row)
    }

    /// Takes the result of a finished Record search and starts one that waited for DataForge,
    /// once `loaded` says loading just ended; returns whether a search finished.
    pub(crate) fn poll_record_search(&mut self, loaded: bool) -> bool {
        if loaded {
            match self.records.as_ref().and_then(RecordTree::shared_index) {
                Some(index) => self.record_search.loaded(index),
                None => self.record_search.stop(),
            }
        }
        self.record_search.poll()
    }

    /// Drops the loaded DataForge because the Archive changed on disk.
    pub(crate) fn drop_records(&mut self) {
        if let Some(records) = &mut self.records {
            records.archive_changed();
        }
        self.record_search.clear();
        self.navigation.archive_changed();
        self.incoming.archive_changed();
    }
}
