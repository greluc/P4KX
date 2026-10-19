use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError, TryLockError};

use p4kx_extraction::DataForgeEntry;

use crate::EntryIndex;
use crate::model::describe;

/// Why the DataForge of a slot could not be read.
#[derive(Debug)]
pub(crate) enum ReadFailure {
    /// The Archive changed on disk before or while it was read.
    ArchiveChanged,
    /// The Entry cannot be read, for the reason given.
    Unreadable(String),
}

/// The DataForge of one open Archive, read by the first run that needs it and kept for every
/// later run, also when the run that read it was cancelled; runs that need it meanwhile wait
/// for that read instead of reading it again.
#[derive(Debug, Default)]
pub(crate) struct ForgeSlot {
    entry: Mutex<Option<Arc<DataForgeEntry>>>,
    reads: AtomicUsize,
}

impl ForgeSlot {
    /// Returns how many reads of the DataForge started, those that failed included.
    pub(crate) fn reads(&self) -> usize {
        self.reads.load(Ordering::Relaxed)
    }

    /// Returns whether the DataForge was read; `false` while it is being read.
    pub(crate) fn is_read(&self) -> bool {
        match self.entry.try_lock() {
            Ok(entry) => entry.is_some(),
            Err(TryLockError::Poisoned(entry)) => entry.into_inner().is_some(),
            Err(TryLockError::WouldBlock) => false,
        }
    }

    /// Returns the DataForge of `index`, reading it unless it was read before; `reading` is
    /// called before a read starts.
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the lock is held through the read so that another run waits for it"
    )]
    pub(crate) fn read(
        &self,
        index: &EntryIndex,
        reading: impl FnOnce(),
    ) -> Result<Arc<DataForgeEntry>, ReadFailure> {
        let mut entry = self.entry.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(entry) = entry.as_ref() {
            return Ok(Arc::clone(entry));
        }
        self.reads.fetch_add(1, Ordering::Relaxed);
        reading();
        let read = DataForgeEntry::read(index.archive())
            .map_err(|error| ReadFailure::Unreadable(describe(&error)))?;
        if index.watch().check().is_err() {
            return Err(ReadFailure::ArchiveChanged);
        }
        let read = Arc::new(read);
        *entry = Some(Arc::clone(&read));
        Ok(read)
    }

    /// Forgets the DataForge read, because the Archive changed on disk.
    pub(crate) fn clear(&self) {
        *self.entry.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }
}
