use std::fmt;
use std::sync::{Arc, Mutex, PoisonError, TryLockError, Weak};

use p4kx_export::RecordFilePlans;
use p4kx_extraction::DataForgeEntry;

use crate::EntryIndex;
use crate::model::describe;

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

/// The [`ForgeSource`] of one open Archive, read by the first export that needs it and kept for
/// every later one, also when the export that read it was given up; exports that need it
/// meanwhile wait for that read instead of reading it again.
#[derive(Debug, Default)]
pub(crate) struct ForgeSlot {
    source: Mutex<Option<Arc<ForgeSource>>>,
}

impl ForgeSlot {
    /// Returns whether the source was read; `false` while it is being read.
    fn is_read(&self) -> bool {
        match self.source.try_lock() {
            Ok(source) => source.is_some(),
            Err(TryLockError::Poisoned(source)) => source.into_inner().is_some(),
            Err(TryLockError::WouldBlock) => false,
        }
    }

    /// Returns the source of `index`, reading DataForge unless it was read before.
    ///
    /// # Errors
    ///
    /// Returns the reason when the Archive changed on disk or the Entry cannot be read.
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the lock is held through the read so that another export waits for it"
    )]
    pub(crate) fn read(&self, index: &EntryIndex) -> Result<Arc<ForgeSource>, String> {
        let mut source = self.source.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(source) = source.as_ref() {
            return Ok(Arc::clone(source));
        }
        let entry = DataForgeEntry::read(index.archive()).map_err(|error| describe(&error))?;
        index.watch().check().map_err(|error| describe(&error))?;
        let read = Arc::new(ForgeSource {
            entry,
            plans: RecordFilePlans::default(),
        });
        *source = Some(Arc::clone(&read));
        Ok(read)
    }
}

/// The [`ForgeSlot`] of the Archive whose index it was made for.
#[derive(Debug, Default)]
pub(crate) struct ForgeCache {
    index: Weak<EntryIndex>,
    slot: Arc<ForgeSlot>,
}

impl ForgeCache {
    /// Returns the slot of `index`, a fresh one when the cache belonged to another Archive.
    pub(crate) fn slot(&mut self, index: &Arc<EntryIndex>) -> Arc<ForgeSlot> {
        if !self.index.ptr_eq(&Arc::downgrade(index)) {
            self.index = Arc::downgrade(index);
            self.slot = Arc::default();
        }
        Arc::clone(&self.slot)
    }

    /// Drops the source, because the Archive changed on disk.
    pub(crate) fn forget(&mut self) {
        self.index = Weak::new();
        self.slot = Arc::default();
    }

    /// Drops the source once the Archive it belongs to is closed.
    pub(crate) fn forget_closed(&mut self) {
        if self.index.strong_count() == 0 && self.slot.is_read() {
            self.forget();
        }
    }

    /// Returns whether the source of `index` is read already.
    pub(crate) fn is_read_for(&self, index: &Arc<EntryIndex>) -> bool {
        self.index.ptr_eq(&Arc::downgrade(index)) && self.slot.is_read()
    }
}
