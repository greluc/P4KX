use std::path::{Path, PathBuf};

use crate::format_error::FormatError;
use crate::source::Source;
use crate::{ArchiveError, Entry, EntryReader, central_directory, end_records, entry_reader};

/// An opened Archive with its Entries.
#[derive(Debug)]
pub struct Archive {
    path: PathBuf,
    source: Source,
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
        let source = Source::open(path).map_err(|error| FormatError::from(error).at(path))?;
        let entries = read_entries(&source).map_err(|error| error.at(path))?;
        Ok(Self {
            path: path.to_path_buf(),
            source,
            entries,
        })
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

    /// Opens the decrypted and decompressed data of `entry`, which must be one of this Archive's
    /// Entries.
    ///
    /// # Errors
    ///
    /// Returns [`ArchiveError::UnsupportedCompression`] for compression methods other than store,
    /// deflate and zstd, and another [`ArchiveError`] when the local header is malformed or cannot
    /// be read, or encrypted data is not a whole number of AES blocks.
    pub fn open_entry(&self, entry: &Entry) -> Result<EntryReader<'_>, ArchiveError> {
        entry_reader::open(&self.source, entry).map_err(|error| error.at(&self.path))
    }
}

fn read_entries(source: &Source) -> Result<Vec<Entry>, FormatError> {
    let location = end_records::locate_directory(source)?;
    let size = usize::try_from(location.size).map_err(|_| {
        FormatError::corrupt(
            location.offset,
            "central directory does not fit into memory",
        )
    })?;
    let directory = source.read_at(location.offset, size)?;
    central_directory::parse_entries(&directory, location.offset, location.entry_count)
}
