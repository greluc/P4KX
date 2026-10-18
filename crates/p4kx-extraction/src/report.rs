use std::io;
use std::path::PathBuf;

use p4kx_archive::ArchiveError;
use thiserror::Error;

/// Why one Entry of an Extraction was not written.
#[derive(Debug, Error)]
pub enum ExtractionError {
    /// The Entry path is absolute, has a drive prefix, names a device or would leave the target.
    #[error("the path would leave the target folder")]
    UnsafePath,
    /// The Entry could not be read from the Archive.
    #[error(transparent)]
    Archive(#[from] ArchiveError),
    /// The Entry data could not be read.
    #[error("cannot read the Entry data")]
    Read(#[source] io::Error),
    /// The file or a folder for the Entry could not be written.
    #[error("cannot write {path}")]
    Write {
        /// The file or folder on disk.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// The file already in the target could not be read to compare it with the Entry.
    #[error("cannot read the existing file {path}")]
    ReadExisting {
        /// The existing file.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
}

/// One Entry that an Extraction did not write.
#[derive(Debug)]
pub struct EntryFailure {
    pub(crate) path: String,
    pub(crate) error: ExtractionError,
}

impl EntryFailure {
    /// Returns the path of the Entry inside the Archive.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns why the Entry was not written.
    #[must_use]
    pub const fn error(&self) -> &ExtractionError {
        &self.error
    }
}

/// The outcome of an Extraction.
#[derive(Debug, Default)]
pub struct ExtractionReport {
    pub(crate) extracted: usize,
    pub(crate) skipped: Vec<String>,
    pub(crate) failures: Vec<EntryFailure>,
    pub(crate) cancelled: Vec<String>,
    pub(crate) archive_changed: bool,
}

impl ExtractionReport {
    /// Returns the number of Entries written.
    #[must_use]
    pub const fn extracted(&self) -> usize {
        self.extracted
    }

    /// Returns the paths of Entries whose existing file in the target was kept.
    #[must_use]
    pub fn skipped(&self) -> &[String] {
        &self.skipped
    }

    /// Returns the Entries that failed, with the reason.
    #[must_use]
    pub fn failures(&self) -> &[EntryFailure] {
        &self.failures
    }

    /// Returns the paths of Entries not written because the run was cancelled.
    #[must_use]
    pub fn cancelled(&self) -> &[String] {
        &self.cancelled
    }

    /// Returns whether the Archive changed on disk before or during the run, which then stopped.
    #[must_use]
    pub const fn archive_changed(&self) -> bool {
        self.archive_changed
    }

    /// Returns whether no Entry failed, the run was not cancelled and the Archive did not change.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.failures.is_empty() && self.cancelled.is_empty() && !self.archive_changed
    }
}
