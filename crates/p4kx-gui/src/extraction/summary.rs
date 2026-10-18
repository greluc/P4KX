use std::path::{Path, PathBuf};

use p4kx_extraction::{EntryFailure, ExtractionReport};

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
    converted: Option<usize>,
    skipped: Vec<String>,
    failures: Vec<FailedEntry>,
    unconverted: Vec<FailedEntry>,
    cancelled: Vec<String>,
    archive_changed: bool,
}

impl ExtractionSummary {
    pub(crate) fn new(target: PathBuf, report: &ExtractionReport) -> Self {
        Self {
            target,
            extracted: report.extracted(),
            converted: report.converted(),
            skipped: report.skipped().to_vec(),
            failures: failed_entries(report.failures()),
            unconverted: failed_entries(report.unconverted()),
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

    /// Returns the number of Entries written converted, `CryXml` as XML and the Entries of
    /// converted textures, or `None` when Conversion was off.
    #[must_use]
    pub const fn converted(&self) -> Option<usize> {
        self.converted
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

    /// Returns the textures, by their header Entry, that could not be converted, with the
    /// reason; their raw Entries were written instead.
    #[must_use]
    pub fn unconverted(&self) -> &[FailedEntry] {
        &self.unconverted
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

fn failed_entries(failures: &[EntryFailure]) -> Vec<FailedEntry> {
    failures
        .iter()
        .map(|failure| FailedEntry {
            path: failure.path().to_owned(),
            reason: describe(failure.error()),
        })
        .collect()
}
