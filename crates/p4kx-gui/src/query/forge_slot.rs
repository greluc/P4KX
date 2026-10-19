use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, TryLockError};

use p4kx_extraction::DataForgeEntry;
use p4kx_extraction::query::QuerySchema;

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
    schema: Mutex<Option<Arc<QuerySchema>>>,
    schema_asked: AtomicBool,
}

impl ForgeSlot {
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

    /// Forgets the DataForge read and its schema, because the Archive changed on disk.
    pub(crate) fn clear(&self) {
        *self.entry.lock().unwrap_or_else(PoisonError::into_inner) = None;
        *self.schema.lock().unwrap_or_else(PoisonError::into_inner) = None;
        self.schema_asked.store(false, Ordering::Release);
    }

    /// Returns the completion schema once it was read.
    pub(crate) fn schema(&self) -> Option<Arc<QuerySchema>> {
        self.schema
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Returns whether this is the first call since the slot was made or cleared, so that the
    /// schema is read once.
    pub(crate) fn ask_schema(&self) -> bool {
        !self.schema_asked.swap(true, Ordering::AcqRel)
    }

    /// Reads the completion schema of `index`, reading its DataForge unless it was read before.
    pub(crate) fn read_schema(&self, index: &EntryIndex) -> Result<(), ReadFailure> {
        let entry = self.read(index, || {})?;
        let forge = entry
            .open()
            .map_err(|error| ReadFailure::Unreadable(describe(&error)))?;
        let schema = Arc::new(QuerySchema::of(&forge));
        *self.schema.lock().unwrap_or_else(PoisonError::into_inner) = Some(schema);
        Ok(())
    }
}
