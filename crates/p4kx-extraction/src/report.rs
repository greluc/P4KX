use std::io;
use std::path::PathBuf;

use p4kx_archive::ArchiveError;
use p4kx_cryxml::CryXmlError;
use p4kx_dataforge::DataForgeError;
use thiserror::Error;

use crate::RecordExportReport;

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
    /// The Entry is `CryXml` that could not be converted.
    #[error("cannot convert the CryXml")]
    Convert(#[source] CryXmlError),
    /// The Entry is `CryXml` or DataForge larger than a Conversion reads into memory.
    #[error("cannot convert the Entry: {size} bytes are more than the limit of {limit}")]
    TooLargeToConvert {
        /// The size of the Entry.
        size: u64,
        /// The largest size converted.
        limit: u64,
    },
    /// The Entry is DataForge that could not be opened to convert its Records.
    #[error("cannot convert the DataForge")]
    DataForge(#[source] DataForgeError),
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
    pub(crate) converted: Option<usize>,
    pub(crate) skipped: Vec<String>,
    pub(crate) failures: Vec<EntryFailure>,
    pub(crate) cancelled: Vec<String>,
    pub(crate) archive_changed: bool,
    pub(crate) records: Option<RecordExportReport>,
}

impl ExtractionReport {
    /// Returns the number of Entries written.
    #[must_use]
    pub const fn extracted(&self) -> usize {
        self.extracted
    }

    /// Returns the number of Entries written converted, `CryXml` as XML and the DataForge Entry as
    /// Records, or `None` when the run did not convert; they count among the
    /// [`extracted`](Self::extracted) Entries.
    #[must_use]
    pub const fn converted(&self) -> Option<usize> {
        self.converted
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

    /// Returns the outcome of the Records a Conversion wrote for the DataForge Entry, or `None`
    /// when no DataForge Entry was converted.
    #[must_use]
    pub const fn records(&self) -> Option<&RecordExportReport> {
        self.records.as_ref()
    }

    /// Returns whether no Entry or Record failed, the run was not cancelled and the Archive did
    /// not change.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        let records = match &self.records {
            Some(records) => records.is_complete(),
            None => true,
        };
        records && self.failures.is_empty() && self.cancelled.is_empty() && !self.archive_changed
    }
}
