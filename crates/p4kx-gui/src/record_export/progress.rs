use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

/// The step a running Record export is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordExportStage {
    /// DataForge is read from the Archive.
    Reading,
    /// The Records to write are chosen.
    Selecting,
    /// The Records are written.
    Writing,
}

/// How far a running Record export has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordExportProgress {
    /// The step it is at.
    pub stage: RecordExportStage,
    /// Records written, kept or failed so far.
    pub done: usize,
    /// Records to write, known once writing starts.
    pub total: usize,
}

/// Whether the run may still be given up while it reads DataForge.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Gate {
    #[default]
    Reading,
    GivenUp,
    Past,
}

#[derive(Debug, Default)]
pub(super) struct Shared {
    stage: AtomicU8,
    pub(super) done: AtomicUsize,
    pub(super) total: AtomicUsize,
    gate: Mutex<Gate>,
}

impl Shared {
    /// Ends the reading of DataForge; returns `false` when the run was given up meanwhile.
    pub(super) fn leave_reading(&self) -> bool {
        let mut gate = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
        if *gate == Gate::GivenUp {
            return false;
        }
        *gate = Gate::Past;
        true
    }

    /// Gives the run up while it still reads DataForge; returns whether it was given up.
    pub(super) fn give_up(&self) -> bool {
        let mut gate = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
        if *gate == Gate::Past {
            return false;
        }
        *gate = Gate::GivenUp;
        true
    }

    pub(super) fn enter(&self, stage: RecordExportStage) {
        self.stage.store(stage as u8, Ordering::Relaxed);
    }

    pub(super) fn progress(&self) -> RecordExportProgress {
        let stage = match self.stage.load(Ordering::Relaxed) {
            0 => RecordExportStage::Reading,
            1 => RecordExportStage::Selecting,
            _ => RecordExportStage::Writing,
        };
        RecordExportProgress {
            stage,
            done: self.done.load(Ordering::Relaxed),
            total: self.total.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Shared;

    #[test]
    fn a_run_given_up_while_reading_does_not_go_on() {
        let shared = Shared::default();

        let given_up = shared.give_up();

        assert!(given_up, "given up while reading");
        assert!(!shared.leave_reading(), "the thread stops after reading");
    }

    #[test]
    fn a_run_past_reading_is_not_given_up() {
        let shared = Shared::default();

        let left = shared.leave_reading();

        assert!(left, "reading ended");
        assert!(!shared.give_up(), "the run is waited for");
    }
}
