use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};
use std::thread;

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
    schema: Mutex<SchemaState>,
}

/// The completion schema of the slot and whether a read of it runs or ran.
#[derive(Debug, Default)]
struct SchemaState {
    schema: Option<Arc<QuerySchema>>,
    asked: bool,
    epoch: u64,
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

    /// Forgets the DataForge read and its schema, because the Archive changed on disk; a schema
    /// read still running is not kept.
    pub(crate) fn clear(&self) {
        *self.entry.lock().unwrap_or_else(PoisonError::into_inner) = None;
        let mut state = self.schema_state();
        state.schema = None;
        state.asked = false;
        state.epoch = state.epoch.wrapping_add(1);
    }

    /// Returns the completion schema once it was read.
    pub(crate) fn schema(&self) -> Option<Arc<QuerySchema>> {
        self.schema_state().schema.clone()
    }

    /// Reads the completion schema of `index` on a background thread, reading its DataForge
    /// unless it was read before, and calls `wake` once it is there. Nothing starts while a read
    /// runs or after one succeeded; after a failed read or spawn the next call tries again.
    pub(crate) fn prepare_schema(
        self: &Arc<Self>,
        index: &Arc<EntryIndex>,
        wake: impl Fn() + Send + 'static,
    ) {
        let epoch = {
            let mut state = self.schema_state();
            if state.asked || state.schema.is_some() {
                return;
            }
            state.asked = true;
            state.epoch
        };
        let slot = Arc::clone(self);
        let thread_index = Arc::clone(index);
        let spawned = thread::Builder::new()
            .name("query-schema".to_owned())
            .spawn(move || {
                let read = slot.read_schema(&thread_index);
                let mut state = slot.schema_state();
                if state.epoch != epoch {
                    return;
                }
                match read {
                    Ok(schema) => {
                        state.schema = Some(schema);
                        drop(state);
                        wake();
                    }
                    Err(ReadFailure::Unreadable(_)) => state.asked = false,
                    Err(ReadFailure::ArchiveChanged) => {}
                }
            });
        if spawned.is_err() {
            self.schema_state().asked = false;
        }
    }

    fn read_schema(&self, index: &EntryIndex) -> Result<Arc<QuerySchema>, ReadFailure> {
        let entry = self.read(index, || {})?;
        let forge = entry
            .open()
            .map_err(|error| ReadFailure::Unreadable(describe(&error)))?;
        Ok(Arc::new(QuerySchema::of(&forge)))
    }

    fn schema_state(&self) -> MutexGuard<'_, SchemaState> {
        self.schema.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
