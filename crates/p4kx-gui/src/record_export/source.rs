use std::fmt;
use std::sync::{Arc, OnceLock, Weak};

use p4kx_export::RecordFilePlans;
use p4kx_extraction::DataForgeEntry;

use crate::EntryIndex;

/// The DataForge of one open Archive as Record exports read it: its Entry, read once, and the
/// files of its Records, planned once per form.
pub(crate) struct ForgeSource {
    pub(crate) entry: DataForgeEntry,
    pub(crate) plans: RecordFilePlans,
}

impl fmt::Debug for ForgeSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForgeSource")
            .field("entry", &self.entry.path())
            .field("plans", &self.plans)
            .finish()
    }
}

/// The [`ForgeSource`] of the Archive whose index it was made for, filled by the first Record
/// export from that Archive.
#[derive(Debug, Default)]
pub(crate) struct ForgeCache {
    index: Weak<EntryIndex>,
    source: Arc<OnceLock<ForgeSource>>,
}

impl ForgeCache {
    /// Returns the slot of the source for `index`, emptied first when it belonged to another
    /// Archive.
    pub(crate) fn slot(&mut self, index: &Arc<EntryIndex>) -> Arc<OnceLock<ForgeSource>> {
        if !self.index.ptr_eq(&Arc::downgrade(index)) {
            self.index = Arc::downgrade(index);
            self.source = Arc::default();
        }
        Arc::clone(&self.source)
    }

    /// Drops the source once the Archive it belongs to is closed.
    pub(crate) fn forget_closed(&mut self) {
        if self.index.strong_count() == 0 && self.source.get().is_some() {
            self.source = Arc::default();
        }
    }

    /// Returns whether the source of `index` is read already.
    pub(crate) fn is_read_for(&self, index: &Arc<EntryIndex>) -> bool {
        self.index.ptr_eq(&Arc::downgrade(index)) && self.source.get().is_some()
    }
}
