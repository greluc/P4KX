use std::ffi::OsString;
use std::path::{MAIN_SEPARATOR_STR, Path, PathBuf};
use std::time::SystemTime;

use crate::container::{self, MAX_CONTAINER_DEPTH};
use crate::format_error::FormatError;
use crate::source::Source;
use crate::{ArchiveError, Build, Entry, EntryReader, entry_reader, layout};

/// An opened Archive with its Entries, or a container opened from one of its Entries.
#[derive(Debug)]
pub struct Archive {
    path: PathBuf,
    location: PathBuf,
    file_len: u64,
    file_modified: Option<SystemTime>,
    depth: usize,
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
            location: path.to_path_buf(),
            file_len: source.len(),
            file_modified: source.modified(),
            depth: 0,
            source,
            entries,
            build,
        })
    }

    /// Opens the container held in `entry`, one of this Archive's Entries, as an Archive of its
    /// own whose Entries are listed and read the same way, without extracting it to disk.
    ///
    /// A stored container is read in place; a compressed or encrypted one is decoded into memory.
    /// The container shares the file, its size, modification time and Build with this Archive.
    /// Containers open up to [`MAX_CONTAINER_DEPTH`] deep, and their Entries are checked against
    /// the other container limits of this crate.
    ///
    /// # Errors
    ///
    /// Returns [`ArchiveError::Container`] naming the Entry, with the reason as its source.
    pub fn open_container(&self, entry: &Entry) -> Result<Self, ArchiveError> {
        let mut location = OsString::from(self.location.as_os_str());
        location.push(MAIN_SEPARATOR_STR);
        location.push(&entry.path);
        let location = PathBuf::from(location);
        match self.read_container(entry) {
            Ok((source, entries)) => Ok(Self {
                path: self.path.clone(),
                location,
                file_len: self.file_len,
                file_modified: self.file_modified,
                depth: self.depth + 1,
                source,
                entries,
                build: self.build.clone(),
            }),
            Err(error) => Err(ArchiveError::Container {
                path: self.location.clone(),
                entry: entry.path.clone(),
                source: Box::new(error.at(&location)),
            }),
        }
    }

    fn read_container(&self, entry: &Entry) -> Result<(Source, Vec<Entry>), FormatError> {
        if self.depth >= MAX_CONTAINER_DEPTH {
            return Err(FormatError::LimitExceeded(format!(
                "containers nest deeper than {MAX_CONTAINER_DEPTH} levels"
            )));
        }
        let source = container::source_of(&self.source, entry)?;
        let entries = container::admit(layout::read_entries(&source)?, source.len())?;
        Ok((source, entries))
    }

    /// Returns the path of the Archive file, also for a container opened from it.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the path of the Archive file followed by the Entry path of every container down to
    /// this one; errors name it.
    #[must_use]
    pub fn location(&self) -> &Path {
        &self.location
    }

    /// Returns the size of the Archive file in bytes when it was opened.
    #[must_use]
    pub const fn file_len(&self) -> u64 {
        self.file_len
    }

    /// Returns the modification time of the Archive file when it was opened, or `None` when the
    /// platform records none.
    #[must_use]
    pub const fn file_modified(&self) -> Option<SystemTime> {
        self.file_modified
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
        entry_reader::open(&self.source, entry).map_err(|error| error.at(&self.location))
    }
}
