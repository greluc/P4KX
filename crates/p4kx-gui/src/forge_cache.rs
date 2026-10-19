use std::fmt;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

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
/// The first consumer that needs DataForge reads it; the others wait for that read instead of
/// reading it again. A read is kept even when the consumer that started it was cancelled, until
/// the Archive changes on disk. [`clear`](Self::clear) never waits for a read.
#[derive(Debug, Default)]
pub(crate) struct ForgeCache {
    reading: Mutex<()>,
    source: Mutex<Option<Arc<ForgeSource>>>,
    generation: AtomicU64,
    reads: AtomicUsize,
}

impl ForgeCache {
    /// Returns whether DataForge was read; `false` while it is being read.
    pub(crate) fn is_read(&self) -> bool {
        self.source
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some()
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
    /// Returns why DataForge cannot be read, or that the Archive changed on disk before or while
    /// it was read.
    pub(crate) fn read(
        &self,
        index: &EntryIndex,
        reading: impl FnOnce(),
    ) -> Result<Arc<ForgeSource>, ReadFailure> {
        let _turn = self.reading.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(source) = self.cached() {
            return Ok(source);
        }
        self.reads.fetch_add(1, Ordering::Relaxed);
        let generation = self.generation.load(Ordering::Acquire);
        let changed = |error| ReadFailure::ArchiveChanged(describe(&error));
        index.watch().check().map_err(changed)?;
        reading();
        let entry = DataForgeEntry::read(index.archive())
            .map_err(|error| ReadFailure::Unreadable(describe(&error)))?;
        index.watch().check().map_err(changed)?;
        let read = Arc::new(ForgeSource {
            entry: Arc::new(entry),
            plans: RecordFilePlans::default(),
        });
        if self.store(generation, &read) {
            Ok(read)
        } else {
            Err(ReadFailure::ArchiveChanged(
                "the DataForge cache was cleared while it was read".to_owned(),
            ))
        }
    }

    /// Forgets the DataForge read, because the Archive changed on disk, and makes a read still
    /// running discard its result; consumers that hold the DataForge keep their copy until they
    /// end. Returns at once, whatever reads.
    pub(crate) fn clear(&self) {
        let mut source = self.source.lock().unwrap_or_else(PoisonError::into_inner);
        self.generation.fetch_add(1, Ordering::AcqRel);
        *source = None;
    }

    fn store(&self, generation: u64, read: &Arc<ForgeSource>) -> bool {
        let mut source = self.source.lock().unwrap_or_else(PoisonError::into_inner);
        let current = self.generation.load(Ordering::Acquire) == generation;
        if current {
            *source = Some(Arc::clone(read));
        }
        current
    }

    fn cached(&self) -> Option<Arc<ForgeSource>> {
        self.source
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[cfg(test)]
mod tests;
