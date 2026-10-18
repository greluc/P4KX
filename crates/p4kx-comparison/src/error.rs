use thiserror::Error;

use crate::{DataForgeError, Guid};

/// Why a Comparison did not produce a result.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ComparisonError {
    /// No level was selected.
    #[error("no Comparison level is selected")]
    NoLevel,
    /// The Record level was selected without the DataForges of both Archives.
    #[error("the Record level needs the DataForges of both Archives")]
    NoDataForge,
    /// The Comparison was cancelled through its [`CancelToken`](crate::CancelToken).
    #[error("the Comparison was cancelled")]
    Cancelled,
    /// A Record of one DataForge cannot be read.
    #[error("the Record {guid} of the {side} DataForge cannot be read")]
    Record {
        /// The side the Record belongs to, `old` or `new`.
        side: &'static str,
        /// The GUID of the Record.
        guid: Guid,
        /// What cannot be read.
        #[source]
        source: DataForgeError,
    },
}

/// Why the line diff of an Entry pair could not be produced.
#[derive(Debug, Error)]
pub enum LineDiffError {
    /// Neither Archive has an Entry with the path.
    #[error("no Entry named {path} in either Archive")]
    UnknownEntry {
        /// The requested path.
        path: String,
    },
    /// The data of an Entry cannot be opened.
    #[error(transparent)]
    Archive(#[from] crate::ArchiveError),
    /// The data of an Entry cannot be read or decompressed.
    #[error("cannot read the Entry {path}")]
    Read {
        /// The Entry path.
        path: String,
        /// What failed.
        #[source]
        source: std::io::Error,
    },
    /// The data of an Entry is longer or shorter than its central-directory size.
    #[error("the Entry {path} does not hold the {declared} bytes its central directory states")]
    SizeMismatch {
        /// The Entry path.
        path: String,
        /// The uncompressed size the central directory states.
        declared: u64,
    },
    /// The `CryXml` of an Entry is truncated or corrupt.
    #[error("cannot convert the CryXml Entry {path}")]
    CryXml {
        /// The Entry path.
        path: String,
        /// What is wrong with it.
        #[source]
        source: p4kx_cryxml::CryXmlError,
    },
}
