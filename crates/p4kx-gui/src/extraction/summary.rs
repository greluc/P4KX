use std::path::{Path, PathBuf};

use p4kx_extraction::ExtractionReport;

use crate::model::describe;

/// One Entry an Extraction did not write, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedEntry {
    /// The path of the Entry inside the Archive.
    pub path: String,
    /// The error and its causes, joined by colons.
    pub reason: String,
}

/// The outcome of a finished Extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionSummary {
    target: PathBuf,
    extracted: usize,
    skipped: Vec<String>,
    failures: Vec<FailedEntry>,
    cancelled: Vec<String>,
    archive_changed: bool,
}

impl ExtractionSummary {
    pub(crate) fn new(target: PathBuf, report: &ExtractionReport) -> Self {
        Self {
            target,
            extracted: report.extracted(),
            skipped: report.skipped().to_vec(),
            failures: report
                .failures()
                .iter()
                .map(|failure| FailedEntry {
                    path: failure.path().to_owned(),
                    reason: describe(failure.error()),
                })
                .collect(),
            cancelled: report.cancelled().to_vec(),
            archive_changed: report.archive_changed(),
        }
    }

    /// Returns the folder the Entries were written into.
    #[must_use]
    pub fn target(&self) -> &Path {
        &self.target
    }

    /// Returns the number of Entries written.
    #[must_use]
    pub const fn extracted(&self) -> usize {
        self.extracted
    }

    /// Returns the paths of Entries whose existing file was kept.
    #[must_use]
    pub fn skipped(&self) -> &[String] {
        &self.skipped
    }

    /// Returns the Entries that failed, with the reason.
    #[must_use]
    pub fn failures(&self) -> &[FailedEntry] {
        &self.failures
    }

    /// Returns the paths of Entries not written because the Extraction was cancelled.
    #[must_use]
    pub fn cancelled(&self) -> &[String] {
        &self.cancelled
    }

    /// Returns whether the Archive changed on disk, which stopped the Extraction.
    #[must_use]
    pub const fn archive_changed(&self) -> bool {
        self.archive_changed
    }
}
