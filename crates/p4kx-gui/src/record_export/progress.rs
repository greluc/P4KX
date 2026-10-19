use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use crate::forge_cache::ReadGate;

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

#[derive(Debug, Default)]
pub(super) struct Shared {
    stage: AtomicU8,
    pub(super) done: AtomicUsize,
    pub(super) total: AtomicUsize,
    gate: ReadGate,
}

impl Shared {
    /// Ends the reading of DataForge; returns `false` when the run was given up meanwhile.
    pub(super) fn leave_reading(&self) -> bool {
        self.gate.leave_reading()
    }

    /// Gives the run up while it still reads DataForge; returns whether it was given up.
    pub(super) fn give_up(&self) -> bool {
        self.gate.give_up()
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
