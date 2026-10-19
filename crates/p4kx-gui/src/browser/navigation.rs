use std::sync::Arc;

use p4kx_extraction::Guid;

use crate::{Browser, RecordId, RecordLink, RecordNodeKind, RecordPreview, TextLanguage};

impl Browser {
    /// Returns how often DataForge of the open Archive was read from the Archive; the Record
    /// tree, the Record preview, the Query tab and the Record export share each read.
    #[must_use]
    pub fn dataforge_reads(&self) -> usize {
        self.index.forge().reads()
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
}
