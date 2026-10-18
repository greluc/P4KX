use std::path::{Path, PathBuf};

use crate::central_directory::{self, DirectoryError};
use crate::end_records::{self, EndRecordsError};
use crate::source::Source;
use crate::{ArchiveError, Entry};

/// An opened Archive with its Entries.
#[derive(Debug)]
pub struct Archive {
    path: PathBuf,
    entries: Vec<Entry>,
}

impl Archive {
    /// Opens the Archive at `path` and reads its central directory.
    ///
    /// The file is opened read-only and stays readable and writable for other processes such as
    /// the launcher and the game.
    ///
    /// # Errors
    ///
    /// Returns an [`ArchiveError`] when the file cannot be read or is not a valid Archive.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ArchiveError> {
        let path = path.as_ref().to_path_buf();
        let io_error = |source| ArchiveError::Io {
            path: path.clone(),
            source,
        };
        let source = Source::open(&path).map_err(io_error)?;
        let location = end_records::locate_directory(&source).map_err(|error| match error {
            EndRecordsError::Io(source) => io_error(source),
            EndRecordsError::NotFound => ArchiveError::NotAnArchive { path: path.clone() },
            EndRecordsError::Corrupt { offset, reason } => ArchiveError::Corrupt {
                path: path.clone(),
                offset,
                reason: reason.to_owned(),
            },
            EndRecordsError::Truncated(source) => ArchiveError::Truncated {
                path: path.clone(),
                source,
            },
        })?;
        let size = usize::try_from(location.size).map_err(|_| ArchiveError::Corrupt {
            path: path.clone(),
            offset: location.offset,
            reason: "central directory does not fit into memory".to_owned(),
        })?;
        let directory = source.read_at(location.offset, size).map_err(io_error)?;
        let entries =
            central_directory::parse_entries(&directory, location.offset, location.entry_count)
                .map_err(|error| match error {
                    DirectoryError::Corrupt { offset, reason } => ArchiveError::Corrupt {
                        path: path.clone(),
                        offset,
                        reason,
                    },
                    DirectoryError::Truncated(source) => ArchiveError::Truncated {
                        path: path.clone(),
                        source,
                    },
                })?;
        Ok(Self { path, entries })
    }

    /// Returns the path the Archive was opened from.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the Entries in central-directory order.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
}
