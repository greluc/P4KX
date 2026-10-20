use std::io;

use thiserror::Error;

use crate::{Checksum, Compression};

/// Why the data of an Entry could not be decoded.
///
/// Reading an [`EntryReader`](crate::EntryReader) reports it inside an [`io::Error`] of kind
/// [`io::ErrorKind::InvalidData`].
#[derive(Debug, Error)]
pub enum DataError {
    /// The compressed data is malformed.
    #[error("{compression} data is corrupt")]
    Corrupt {
        /// The compression method of the Entry.
        compression: Compression,
        /// What the decompressor reported.
        #[source]
        source: io::Error,
    },
    /// The data ended before the uncompressed size given in the central directory.
    #[error("Entry data ends after {actual} of {expected} bytes")]
    TooShort {
        /// Number of bytes the data yielded.
        actual: u64,
        /// The uncompressed size from the central directory.
        expected: u64,
    },
    /// The data goes on after the uncompressed size given in the central directory; reported only
    /// by a reader that [rejects overlong data](crate::EntryReader::reject_overlong).
    #[error("Entry data holds more than its {expected} bytes")]
    TooLong {
        /// The uncompressed size from the central directory.
        expected: u64,
    },
    /// The checksum of the decoded data differs from the one in the directory; reported only by
    /// a reader from [`Archive::open_entry_verified`](crate::Archive::open_entry_verified), at
    /// the end of the data.
    #[error("{path}: {actual} computed, {expected} stored")]
    ChecksumMismatch {
        /// The path of the Entry.
        path: String,
        /// The checksum the directory states.
        expected: Checksum,
        /// The checksum of the decoded data.
        actual: Checksum,
    },
}

impl From<DataError> for io::Error {
    fn from(error: DataError) -> Self {
        Self::new(io::ErrorKind::InvalidData, error)
    }
}
