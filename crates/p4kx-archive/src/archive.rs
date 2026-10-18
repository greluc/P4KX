use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::format_error::FormatError;

/// The most decoded bytes [`Archive::open_entry_at`] skips in a compressed Entry to reach an offset.
pub const MAX_DECODED_SKIP: u64 = 1 << 30;
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
        self.open_entry_at(entry, 0)
    }

    /// Opens the decrypted and decompressed data of `entry` from byte `offset` on.
    ///
    /// Stored Entries start there directly, encrypted ones at the AES block holding `offset`.
    /// Compressed Entries are decoded from their start, and the bytes before `offset` are skipped,
    /// at most [`MAX_DECODED_SKIP`].
    ///
    /// # Errors
    ///
    /// Returns [`ArchiveError::OffsetBeyondEnd`] when `offset` exceeds the Entry size,
    /// [`ArchiveError::SkipTooLong`] when a compressed Entry would decode more than
    /// [`MAX_DECODED_SKIP`] bytes to reach `offset`, and the errors of [`Archive::open_entry`],
    /// or [`ArchiveError::Io`] when the skipped data cannot be read or decoded.
    pub fn open_entry_at(
        &self,
        entry: &Entry,
        offset: u64,
    ) -> Result<EntryReader<'_>, ArchiveError> {
        let size = entry.uncompressed_size();
        if offset > size {
            return Err(ArchiveError::OffsetBeyondEnd {
                path: self.path.clone(),
                entry: entry.path().to_owned(),
                offset,
                size,
            });
        }
        let (mut reader, skip) = entry_reader::open_at(&self.source, entry, offset)
            .map_err(|error| error.at(&self.path))?;
        if skip > MAX_DECODED_SKIP {
            return Err(ArchiveError::SkipTooLong {
                path: self.path.clone(),
                entry: entry.path().to_owned(),
                offset,
                limit: MAX_DECODED_SKIP,
            });
        }
        io::copy(&mut (&mut reader).take(skip), &mut io::sink()).map_err(|source| {
            ArchiveError::Io {
                path: self.path.clone(),
                source,
            }
        })?;
        Ok(reader)
    }
}
