use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError, TryLockError};

use p4kx_export::RecordFilePlans;
use p4kx_extraction::DataForgeEntry;

use crate::EntryIndex;
use crate::model::describe;

/// The DataForge of one open Archive as every consumer reads it: its Entry, read once, and the
/// files of its Records, planned once per form.
pub(crate) struct ForgeSource {
    pub(crate) entry: Arc<DataForgeEntry>,
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

/// Why the DataForge of an Archive could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReadFailure {
    /// The Archive changed on disk before or while it was read, for the reason given.
    ArchiveChanged(String),
    /// The Entry cannot be read, for the reason given.
    Unreadable(String),
}

impl ReadFailure {
    /// Returns the reason.
    pub(crate) fn into_reason(self) -> String {
        match self {
            Self::ArchiveChanged(reason) | Self::Unreadable(reason) => reason,
        }
    }
}

/// The one DataForge cache of an open Archive, shared by the Record tree, the Record preview, the
/// Query tab and the Record export.
///
/// The first consumer that needs DataForge reads it under the lock; the others wait for that read
/// instead of reading it again. A read is kept even when the consumer that started it was
/// cancelled, until the Archive changes on disk.
#[derive(Debug, Default)]
pub(crate) struct ForgeCache {
    source: Mutex<Option<Arc<ForgeSource>>>,
    reads: AtomicUsize,
}

impl ForgeCache {
    /// Returns whether DataForge was read; `false` while it is being read.
    pub(crate) fn is_read(&self) -> bool {
        match self.source.try_lock() {
            Ok(source) => source.is_some(),
            Err(TryLockError::Poisoned(source)) => source.into_inner().is_some(),
            Err(TryLockError::WouldBlock) => false,
        }
    }

    /// Returns how often DataForge was read from the Archive, whether or not the read succeeded.
    pub(crate) fn reads(&self) -> usize {
        self.reads.load(Ordering::Relaxed)
    }

    /// Returns the DataForge of `index`, reading it unless it was read before; `reading` is
    /// called before a read starts.
    ///
    /// # Errors
    ///
    /// Returns why DataForge cannot be read, or that the Archive changed on disk while it was.
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the lock is held through the read so that another consumer waits for it"
    )]
    pub(crate) fn read(
        &self,
        index: &EntryIndex,
        reading: impl FnOnce(),
    ) -> Result<Arc<ForgeSource>, ReadFailure> {
        let mut source = self.source.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(source) = source.as_ref() {
            return Ok(Arc::clone(source));
        }
        reading();
        self.reads.fetch_add(1, Ordering::Relaxed);
        let entry = DataForgeEntry::read(index.archive())
            .map_err(|error| ReadFailure::Unreadable(describe(&error)))?;
        index
            .watch()
            .check()
            .map_err(|error| ReadFailure::ArchiveChanged(describe(&error)))?;
        let read = Arc::new(ForgeSource {
            entry: Arc::new(entry),
            plans: RecordFilePlans::default(),
        });
        *source = Some(Arc::clone(&read));
        Ok(read)
    }

    /// Forgets the DataForge read, because the Archive changed on disk; consumers that hold it
    /// keep their copy until they end.
    pub(crate) fn clear(&self) {
        *self.source.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }
}
