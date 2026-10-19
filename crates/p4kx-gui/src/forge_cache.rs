//! The DataForge of the open Archive, read once and shared by the Record export and the Basetool
//! catalog.

use std::fmt;
use std::sync::{Arc, Mutex, PoisonError, TryLockError, Weak};

use p4kx_export::RecordFilePlans;
use p4kx_extraction::DataForgeEntry;

use crate::EntryIndex;
use crate::model::describe;

/// The DataForge of one open Archive as the exports read it: its Entry, read once, and the files
/// of its Records, planned once per form.
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

/// Whether a run may still be given up while it reads DataForge.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Gate {
    #[default]
    Reading,
    GivenUp,
    Past,
}

/// Decides between the window giving a run up while it reads DataForge and the run's thread
/// leaving the read step, so that a run given up never goes on.
#[derive(Debug, Default)]
pub(crate) struct ReadGate {
    gate: Mutex<Gate>,
}

impl ReadGate {
    /// Ends the reading of DataForge; returns `false` when the run was given up meanwhile.
    pub(crate) fn leave_reading(&self) -> bool {
        let mut gate = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
        if *gate == Gate::GivenUp {
            return false;
        }
        *gate = Gate::Past;
        true
    }

    /// Gives the run up while it still reads DataForge; returns whether it was given up.
    pub(crate) fn give_up(&self) -> bool {
        let mut gate = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
        if *gate == Gate::Past {
            return false;
        }
        *gate = Gate::GivenUp;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::ReadGate;

    #[test]
    fn a_run_given_up_while_reading_does_not_go_on() {
        let gate = ReadGate::default();

        let given_up = gate.give_up();

        assert!(given_up, "given up while reading");
        assert!(!gate.leave_reading(), "the thread stops after reading");
    }

    #[test]
    fn a_run_past_reading_is_not_given_up() {
        let gate = ReadGate::default();

        let left = gate.leave_reading();

        assert!(left, "reading ended");
        assert!(!gate.give_up(), "the run is waited for");
    }
}
