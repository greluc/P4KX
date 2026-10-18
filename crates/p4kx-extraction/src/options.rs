use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::ArchiveWatch;

/// What an Extraction does with a file that already exists in the target.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ConflictPolicy {
    /// Keeps the existing file and counts the Entry as skipped.
    #[default]
    Skip,
    /// Replaces the existing file.
    Overwrite,
    /// Replaces the existing file only when its size or CRC-32C differs from the Entry's.
    IfChanged,
}

/// A shared flag that asks a running Extraction to stop.
///
/// Clones share the flag, so one clone can be handed to a signal handler or another thread.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// Creates a token that is not cancelled.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Asks every Extraction holding this token to stop.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    /// Returns whether [`cancel`](Self::cancel) was called.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// How far an Extraction has come.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    /// Entries that ended: written, skipped, failed or cancelled.
    pub entries_done: usize,
    /// Entries of the run.
    pub entries_total: usize,
    /// Uncompressed bytes written so far, plus the full size of every Entry that ended.
    pub bytes_done: u64,
    /// Uncompressed size of every Entry of the run.
    pub bytes_total: u64,
}

/// The settings of one Extraction.
#[derive(Clone, Copy, Default)]
pub struct ExtractOptions<'a> {
    /// What to do with files that already exist.
    pub policy: ConflictPolicy,
    /// Converts `CryXml` Entries to UTF-8 XML; text XML and every other Entry stay unchanged.
    pub convert: bool,
    /// Stops the run once cancelled.
    pub cancel: Option<&'a CancelToken>,
    /// Called from any worker thread whenever an Entry ends or a chunk of data is written.
    pub progress: Option<&'a (dyn Fn(Progress) + Sync)>,
    /// Checked before the run, periodically during it and after it; a change stops the run.
    pub watch: Option<&'a ArchiveWatch>,
}

impl fmt::Debug for ExtractOptions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExtractOptions")
            .field("policy", &self.policy)
            .field("convert", &self.convert)
            .field("cancel", &self.cancel)
            .field("progress", &self.progress.is_some())
            .field("watch", &self.watch)
            .finish()
    }
}
