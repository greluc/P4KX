use std::path::{Path, PathBuf};

use p4kx_extraction::{ExtractionReport, RecordExportReport};

use crate::model::describe;

/// One Entry an Extraction did not write, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedEntry {
    /// The path of the Entry inside the Archive.
    pub path: String,
    /// The error and its causes, joined by colons.
    pub reason: String,
}

/// The counts of the Records a Conversion wrote for the DataForge Entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordCounts {
    /// Records written.
    pub written: usize,
    /// Records whose existing file was kept.
    pub skipped: usize,
    /// Records not written because they failed.
    pub failed: usize,
    /// Records not written because the Extraction was cancelled.
    pub cancelled: usize,
    /// Error values in the Records written or kept.
    pub error_values: usize,
    /// Records written or kept that hold at least one error value.
    pub records_with_errors: usize,
}

impl RecordCounts {
    fn new(report: &RecordExportReport) -> Self {
        Self {
            written: report.written(),
            skipped: report.skipped().len(),
            failed: report.failures().len(),
            cancelled: report.cancelled(),
            error_values: report.error_values(),
            records_with_errors: report.records_with_errors(),
        }
    }
}

/// The outcome of a finished Extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionSummary {
    target: PathBuf,
    extracted: usize,
    converted: Option<usize>,
    skipped: Vec<String>,
    failures: Vec<FailedEntry>,
    cancelled: Vec<String>,
    archive_changed: bool,
    records: Option<RecordCounts>,
}

impl ExtractionSummary {
    pub(crate) fn new(target: PathBuf, report: &ExtractionReport) -> Self {
        Self {
            target,
            extracted: report.extracted(),
            converted: report.converted(),
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
            records: report.records().map(RecordCounts::new),
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

    /// Returns the number of `CryXml` Entries written as XML, or `None` when Conversion was off.
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

    /// Returns the paths of Entries not written because the Extraction was cancelled.
    #[must_use]
    pub fn cancelled(&self) -> &[String] {
        &self.cancelled
    }

    /// Returns the counts of the Records written for the DataForge Entry, or `None` when no
    /// DataForge Entry was converted.
    #[must_use]
    pub const fn records(&self) -> Option<RecordCounts> {
        self.records
    }

    /// Returns whether the Archive changed on disk, which stopped the Extraction.
    #[must_use]
    pub const fn archive_changed(&self) -> bool {
        self.archive_changed
    }
}
