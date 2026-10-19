use thiserror::Error;

use crate::Entry;

/// A defect of one Entry that does not keep the Archive from opening.
///
/// The Entry stays listed. Reading it fails with an [`ArchiveError`](crate::ArchiveError) only
/// where the defect makes its data unreadable.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum EntryIssue {
    /// The Entry's data lies outside the file or behind the directory; reading it fails.
    #[error("its data lies outside the file")]
    DataOutsideFile,
    /// The encryption field is neither 0 nor 1; the Entry is read as encrypted.
    #[error("its encryption field is {value}, neither 0 nor 1; it is read as encrypted")]
    UnknownEncryption {
        /// The value of the field.
        value: u16,
    },
    /// The CIG encryption block is not 6 bytes long; its value is read as far as it goes.
    #[error("its encryption block is {size} bytes long, not 6")]
    EncryptionBlockSize {
        /// The block length as stored, header included.
        size: u16,
    },
    /// The path is not UTF-8; [`Entry::path`] holds it with invalid bytes replaced.
    #[error("its path is not UTF-8")]
    PathNotUtf8 {
        /// The path bytes as stored.
        raw: Box<[u8]>,
    },
}

/// A defect together with the position of the Entry it belongs to.
pub(crate) type Issues = Vec<(usize, EntryIssue)>;

/// Decodes the stored path bytes of the Entry at `index`; a path that is not UTF-8 is decoded
/// lossily and recorded in `issues` with its bytes.
pub(crate) fn decode_path(bytes: &[u8], index: usize, issues: &mut Issues) -> String {
    std::str::from_utf8(bytes).map_or_else(
        |_| {
            issues.push((index, EntryIssue::PathNotUtf8 { raw: bytes.into() }));
            String::from_utf8_lossy(bytes).into_owned()
        },
        str::to_owned,
    )
}

/// The Entries of a directory and the defects found while reading it.
pub(crate) struct Directory {
    pub(crate) entries: Vec<Entry>,
    pub(crate) issues: Issues,
}
