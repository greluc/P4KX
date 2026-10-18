use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::format_error::FormatError;
use crate::source::Source;
use crate::{ArchiveError, Build, Entry, EntryReader, entry_reader, layout};

/// An opened Archive with its Entries.
#[derive(Debug)]
pub struct Archive {
    path: PathBuf,
    source: Source,
    entries: Vec<Entry>,
    build: Option<Build>,
}

impl Archive {
    /// Opens the Archive at `path` and reads its central directory, in the v1 or the v2 layout.
    ///
    /// The file is opened read-only and stays readable and writable for other processes such as
    /// the launcher and the game. The Build is read from the `build_manifest.id` next to the
    /// file; a missing or unreadable manifest leaves it unknown.
    ///
    /// # Errors
    ///
    /// Returns an [`ArchiveError`] when the file cannot be read or is not a valid Archive.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ArchiveError> {
        let path = path.as_ref();
        let source = Source::open(path).map_err(|error| FormatError::from(error).at(path))?;
        let entries = layout::read_entries(&source).map_err(|error| error.at(path))?;
        let build = path
            .parent()
            .and_then(|folder| Build::read_from(folder).ok().flatten());
        Ok(Self {
            path: path.to_path_buf(),
            source,
            entries,
            build,
        })
    }

    /// Returns the path the Archive was opened from.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the size of the file in bytes when it was opened.
    #[must_use]
    pub const fn file_len(&self) -> u64 {
        self.source.len()
    }

    /// Returns the modification time of the file when it was opened, or `None` when the platform
    /// records none.
    #[must_use]
    pub const fn file_modified(&self) -> Option<SystemTime> {
        self.source.modified()
    }

    /// Returns the Entries in central-directory order.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Returns the Build, or `None` when it is unknown.
    #[must_use]
    pub const fn build(&self) -> Option<&Build> {
        self.build.as_ref()
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
