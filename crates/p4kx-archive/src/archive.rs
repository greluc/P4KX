use std::path::Path;

use crate::format_error::FormatError;
use crate::source::Source;
use crate::{ArchiveError, Entry, central_directory, end_records};

/// An opened Archive with its Entries.
#[derive(Debug)]
pub struct Archive {
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
        let path = path.as_ref();
        read_entries(path)
            .map(|entries| Self { entries })
            .map_err(|error| error.at(path))
    }

    /// Returns the Entries in central-directory order.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
}

fn read_entries(path: &Path) -> Result<Vec<Entry>, FormatError> {
    let source = Source::open(path)?;
    let location = end_records::locate_directory(&source)?;
    let size = usize::try_from(location.size).map_err(|_| {
        FormatError::corrupt(
            location.offset,
            "central directory does not fit into memory",
        )
    })?;
    let directory = source.read_at(location.offset, size)?;
    central_directory::parse_entries(&directory, location.offset, location.entry_count)
}
