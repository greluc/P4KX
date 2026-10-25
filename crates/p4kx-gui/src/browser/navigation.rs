// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use p4kx_extraction::Guid;

use crate::{
    Browser, IncomingPreview, RecordId, RecordLink, RecordNodeKind, RecordPreview, TextLanguage,
    TreeNode,
};

impl Browser {
    /// Returns how often DataForge of the open Archive was read from the Archive; the Record
    /// tree, the Record preview, the Query tab and the Record export share each read.
    #[must_use]
    pub fn dataforge_reads(&self) -> usize {
        self.index.forge().reads()
    }

    /// Returns how often the Query completion schema was built for the DataForge of the open
    /// Archive; it is built from the DataForge already read, once per Archive.
    #[must_use]
    pub fn query_schema_builds(&self) -> usize {
        self.index.forge().schema_builds()
    }

    /// Returns the Record selected in the Record tree, or `None` when a folder, a record folder
    /// or nothing of the tree is selected.
    #[must_use]
    pub fn selected_record(&self) -> Option<RecordId> {
        let tree = self.records()?;
        match tree.node(tree.selected()?).kind() {
            RecordNodeKind::Record(id) => Some(id),
            RecordNodeKind::Folder | RecordNodeKind::RecordFolder => None,
        }
    }

    /// Returns the preview of the selected Record, or `None` when no Record is selected or its
    /// preview was not asked for yet.
    #[must_use]
    pub fn record_preview(&self) -> Option<&RecordPreview> {
        let selected = self.selected_record()?;
        self.navigation
            .preview()
            .filter(|preview| preview.record() == selected)
    }

    /// Returns the list of the Records that refer to the selected Record, or None when no Record
    /// is selected or the list was not asked for yet.
    #[must_use]
    pub fn incoming_preview(&self) -> Option<&IncomingPreview> {
        let selected = self.selected_record()?;
        self.incoming
            .preview()
            .filter(|preview| preview.record() == selected)
    }

    /// Finds the Records that refer to the selected Record on the background thread of the list
    /// unless that is done or running; the first search of an Archive builds the index of its
    /// References, which takes a moment, and every later one uses it. `wake` is called from the
    /// thread once the list is there; the thread keeps the `wake` of the first call.
    pub fn load_incoming(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        let Some(id) = self.selected_record() else {
            return;
        };
        if self.incoming.requests(id) {
            return;
        }
        self.incoming.request(id, &self.index, Arc::new(wake));
    }

    /// Returns how often the index of References was built for the DataForge of the open Archive;
    /// it is built once per Archive, on the first request of the list.
    #[must_use]
    pub fn reference_builds(&self) -> usize {
        self.index.forge().reference_builds()
    }

    /// Returns the language Record previews resolve localisation keys in, or `None` for the keys
    /// alone.
    #[must_use]
    pub const fn record_language(&self) -> Option<TextLanguage> {
        self.record_language
    }

    /// Makes Record previews resolve localisation keys in `language`, or show the keys alone for
    /// `None`; the preview of the selected Record follows with the next
    /// [`Browser::load_record_preview`].
    pub const fn set_record_language(&mut self, language: Option<TextLanguage>) {
        self.record_language = language;
    }

    /// Reads the selected Record for the preview on a background thread unless it is previewed
    /// already in the language of [`Browser::record_language`]; `wake` is called from that thread
    /// once it is read. The Record stays on screen while a language change reads it again.
    pub fn load_record_preview(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        let Some(id) = self.selected_record() else {
            return;
        };
        if self.navigation.previews(id, self.record_language) {
            return;
        }
        self.navigation
            .show(id, self.record_language, &self.index, Arc::new(wake));
    }

    /// Returns the GUID of the missing Record that the last followed Reference points at, until
    /// another Record is selected.
    #[must_use]
    pub const fn dangling_reference(&self) -> Option<Guid> {
        self.navigation.dangling()
    }

    /// Follows `link`: selects its target Record in the Record tree, which the preview then
    /// shows, and makes it the next step of the way taken; a link whose target does not exist
    /// only notes that, see [`Browser::dangling_reference`].
    pub fn follow(&mut self, link: &RecordLink) {
        match link.target() {
            Some(target)
                if self
                    .records()
                    .and_then(|tree| tree.record(target))
                    .is_some() =>
            {
                self.navigation.visit(target);
                self.pick_record_quietly(target);
            }
            _ => self.navigation.follow_dangling(link.guid()),
        }
    }

    /// Notes that a Reference to the missing Record `guid` was followed, see
    /// [`Browser::dangling_reference`].
    pub const fn follow_dangling(&mut self, guid: Guid) {
        self.navigation.follow_dangling(guid);
    }

    /// Returns whether [`Browser::go_back`] would select a Record.
    #[must_use]
    pub fn can_go_back(&self) -> bool {
        self.navigation.can_go_back() || self.away_from_current()
    }

    /// Returns whether [`Browser::go_forward`] would select a Record.
    #[must_use]
    pub fn can_go_forward(&self) -> bool {
        self.navigation.can_go_forward()
    }

    /// Selects the Record before the current one on the way taken, or the current one again when
    /// another node is selected; returns whether a Record was selected.
    pub fn go_back(&mut self) -> bool {
        let id = if self.away_from_current() {
            self.navigation.current()
        } else {
            self.navigation.back()
        };
        id.is_some_and(|id| self.pick_record_quietly(id))
    }

    /// Selects the Record after the current one on the way taken; returns whether one was
    /// selected.
    pub fn go_forward(&mut self) -> bool {
        self.navigation
            .forward()
            .is_some_and(|id| self.pick_record_quietly(id))
    }

    fn away_from_current(&self) -> bool {
        self.navigation
            .current()
            .is_some_and(|current| self.selected_record() != Some(current))
    }

    /// Selects and reveals the Record `id` in the tree without a step on the way taken.
    fn pick_record_quietly(&mut self, id: RecordId) -> bool {
        self.records.as_mut().is_some_and(|tree| tree.reveal(id))
    }

    /// Shows the Entry at `path` of the Archive: ends a search, selects the folder that holds it
    /// with the folders above it expanded, and selects the Entry for the preview, which is read
    /// on a background thread calling `wake`. Returns whether the Archive holds the Entry.
    pub fn reveal_entry(
        &mut self,
        (path, line): (&str, Option<u64>),
        wake: impl Fn() + Send + Sync + 'static,
    ) -> bool {
        let index = Arc::clone(&self.index);
        let Some(id) = index.id_of_path(path) else {
            return false;
        };
        let parent = path
            .rsplit_once(['\\', '/'])
            .map_or("", |(parent, _)| parent);
        let Some(folder) = index.find_folder(parent) else {
            return false;
        };
        for (at, _) in parent.match_indices(['\\', '/']) {
            if let Some(above) = parent.get(..at).and_then(|above| index.find_folder(above)) {
                self.expanded.insert(TreeNode::from(above));
            }
        }
        self.select_folder(folder);
        self.select_entry(id, wake);
        self.hit_line = line.map(|line| HitLine::new(id, line));
        true
    }

    /// Returns the line the preview of the Entry scrolls to once it shows the text, after
    /// [`reveal_entry`](Self::reveal_entry) was given one.
    #[must_use]
    pub const fn hit_line(&self) -> Option<HitLine> {
        self.hit_line
    }
}

static HIT_SERIAL: AtomicU64 = AtomicU64::new(1);

/// The line of a content-search hit in the Entry with id `entry`, one-based as in the hit.
///
/// Each one has a serial no other has, so that a view scrolls once for each opened hit, also when
/// two hits are in the same Entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitLine {
    serial: u64,
    entry: u32,
    line: u64,
}

impl HitLine {
    fn new(entry: u32, line: u64) -> Self {
        Self {
            serial: HIT_SERIAL.fetch_add(1, Ordering::Relaxed),
            entry,
            line,
        }
    }

    /// Returns a number no other hit line of this process has.
    #[must_use]
    pub const fn serial(&self) -> u64 {
        self.serial
    }

    /// Returns the id of the Entry.
    #[must_use]
    pub const fn entry(&self) -> u32 {
        self.entry
    }

    /// Returns the zero-based index of the line, `0` for a hit without a line.
    #[must_use]
    pub const fn row(&self) -> u64 {
        self.line.saturating_sub(1)
    }
}
