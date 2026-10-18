use crate::{Browser, RecordNodeId, RecordTree};

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
        }
    }

    /// Loads DataForge on a background thread unless it is loaded or loading already, calling
    /// `wake` from it once it is ready.
    pub fn load_records(&mut self, wake: impl Fn() + Send + 'static) {
        if let Some(records) = &mut self.records {
            records.load(&self.index, wake);
        }
    }

    /// Drops the loaded DataForge because the Archive changed on disk.
    pub(crate) fn drop_records(&mut self) {
        if let Some(records) = &mut self.records {
            records.archive_changed();
        }
    }
}
