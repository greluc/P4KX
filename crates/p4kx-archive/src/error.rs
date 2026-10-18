use std::io;
use std::path::PathBuf;

use p4kx_foundation::ParseError;

use crate::Compression;
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
    /// The Archive is written in a layout this version cannot read.
    #[error("{path} uses Archive layout v{version}, which is not supported")]
    UnsupportedLayout {
        /// Path of the Archive.
        path: PathBuf,
        /// The layout version from the Archive's trailer.
        version: u16,
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
    /// The Entry uses a compression method this version cannot read yet.
    #[error("{path}: Entry {entry} uses {compression} compression, which is not supported yet")]
    UnsupportedCompression {
        /// Path of the Archive.
        path: PathBuf,
        /// Path of the Entry inside the Archive.
        entry: String,
        /// The compression method of the Entry.
        compression: Compression,
    },
}
