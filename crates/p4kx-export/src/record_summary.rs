use p4kx_extraction::{ExtractionError, RecordExportReport, RecordFailure};

use crate::RecordForm;

/// The outcome of a Record export.
#[derive(Debug, Default)]
pub struct RecordExportSummary {
    pub(crate) form: RecordForm,
    pub(crate) written: usize,
    pub(crate) skipped: Vec<String>,
    pub(crate) failures: Vec<RecordFailure>,
    pub(crate) cancelled: usize,
    pub(crate) error_values: usize,
    pub(crate) records_with_errors: usize,
    pub(crate) error: Option<ExtractionError>,
}

impl RecordExportSummary {
    pub(crate) fn from_report(form: RecordForm, report: RecordExportReport) -> Self {
        let mut summary = Self {
            form,
            written: report.written(),
            skipped: report.skipped().to_vec(),
            cancelled: report.cancelled(),
            error_values: report.error_values(),
            records_with_errors: report.records_with_errors(),
            ..Self::default()
        };
        summary.failures = report.into_failures();
        summary
    }

    /// Returns the form the Records were written in.
    #[must_use]
    pub const fn form(&self) -> RecordForm {
        self.form
    }

    /// Returns the number of Records written: files for one file per Record, lines for NDJSON.
    #[must_use]
    pub const fn written(&self) -> usize {
        self.written
    }

    /// Returns the files, relative to the target, whose existing file was kept.
    #[must_use]
    pub fn skipped(&self) -> &[String] {
        &self.skipped
    }

    /// Returns the Records that failed, with the reason.
    #[must_use]
    pub fn failures(&self) -> &[RecordFailure] {
        &self.failures
    }

    /// Returns the number of Records not written because the run was cancelled.
    #[must_use]
    pub const fn cancelled(&self) -> usize {
        self.cancelled
    }

    /// Returns the number of error values in the Records written or kept.
    #[must_use]
    pub const fn error_values(&self) -> usize {
        self.error_values
    }

    /// Returns the number of Records written or kept that hold at least one error value.
    #[must_use]
    pub const fn records_with_errors(&self) -> usize {
        self.records_with_errors
    }

    /// Returns why the NDJSON file could not be written, if it could not.
    #[must_use]
    pub const fn error(&self) -> Option<&ExtractionError> {
        self.error.as_ref()
    }

    /// Returns whether no Record failed, the file was written and the run was not cancelled.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.failures.is_empty() && self.cancelled == 0 && self.error.is_none()
    }
}
