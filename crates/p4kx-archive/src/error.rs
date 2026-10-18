use std::io;
use std::path::PathBuf;

use p4kx_foundation::ParseError;
use thiserror::Error;

/// A failure to open or read an Archive.
#[derive(Debug, Error)]
pub enum ArchiveError {
    /// The Archive file could not be read.
    #[error("cannot read {path}")]
    Io {
        /// Path of the Archive.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// The file has no end-of-central-directory record, so it is not an Archive.
    #[error("{path} is not a p4k Archive: no end of central directory found")]
    NotAnArchive {
        /// Path of the file.
        path: PathBuf,
    },
    /// A structure of the Archive is malformed.
    #[error("{path} is corrupt at offset {offset}: {reason}")]
    Corrupt {
        /// Path of the Archive.
        path: PathBuf,
        /// File offset of the malformed structure.
        offset: u64,
        /// What is wrong.
        reason: String,
    },
    /// A structure of the Archive ended early.
    #[error("{path} is truncated or corrupt")]
    Truncated {
        /// Path of the Archive.
        path: PathBuf,
        /// Where the data ran out.
        #[source]
        source: ParseError,
    },
}
