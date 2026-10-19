use std::io::{self, Read};

use p4kx_archive::{Archive, ArchiveError, Entry};
use p4kx_dataforge::{DataForge, DataForgeError};
use thiserror::Error;

use crate::ArchiveChanged;

/// The Entries that may hold DataForge, the preferred one first.
pub const DATAFORGE_ENTRIES: [&str; 2] = ["Data\\Game2.dcb", "Data\\Game.dcb"];

/// The largest uncompressed DataForge Entry that is read, 2 GiB; larger sizes are rejected.
pub const MAX_DATAFORGE_SIZE: u64 = 2 << 30;

/// The bytes of an Archive's DataForge Entry, read into memory.
#[derive(Debug)]
pub struct DataForgeEntry {
    path: String,
    bytes: Vec<u8>,
}

impl DataForgeEntry {
    /// Reads the DataForge Entry of `archive`: `Data\Game2.dcb`, or `Data\Game.dcb` when the
    /// Archive has no `Game2.dcb`.
    ///
    /// # Errors
    ///
    /// Returns [`DataForgeLoadError::Missing`] when the Archive holds neither Entry,
    /// [`DataForgeLoadError::TooLarge`] when its size exceeds [`MAX_DATAFORGE_SIZE`], and another
    /// [`DataForgeLoadError`] when the Entry cannot be read.
    pub fn read(archive: &Archive) -> Result<Self, DataForgeLoadError> {
        let entry = Self::locate(archive).ok_or(DataForgeLoadError::Missing)?;
        let path = entry.path().to_owned();
        let size = entry.uncompressed_size();
        if size > MAX_DATAFORGE_SIZE {
            return Err(DataForgeLoadError::TooLarge {
                path,
                size,
                limit: MAX_DATAFORGE_SIZE,
            });
        }
        let mut bytes = Vec::new();
        let reserve = usize::try_from(size).unwrap_or(usize::MAX);
        if bytes.try_reserve_exact(reserve).is_err() {
            return Err(DataForgeLoadError::TooLarge {
                path,
                size,
                limit: MAX_DATAFORGE_SIZE,
            });
        }
        let read = archive
            .open_entry(entry)
            .map_err(DataForgeLoadError::Archive)
            .and_then(|reader| {
                reader.take(size).read_to_end(&mut bytes).map_err(|source| {
                    DataForgeLoadError::Read {
                        path: path.clone(),
                        source,
                    }
                })
            });
        read.map(|_| {
            bytes.shrink_to_fit();
            Self { path, bytes }
        })
    }

    /// Returns the Entry [`DataForgeEntry::read`] reads, or `None` when the Archive holds no
    /// DataForge; case and separator kind of the path do not matter.
    #[must_use]
    pub fn locate(archive: &Archive) -> Option<&Entry> {
        DATAFORGE_ENTRIES.iter().find_map(|wanted| {
            archive
                .entries()
                .iter()
                .find(|entry| same_path(entry.path(), wanted))
        })
    }

    /// Returns the path of the Entry the DataForge was read from.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Opens the DataForge over the Entry's bytes.
    ///
    /// # Errors
    ///
    /// Returns a [`DataForgeError`] for an unsupported version or a corrupt database.
    pub fn open(&self) -> Result<DataForge<'_>, DataForgeError> {
        DataForge::open(&self.bytes)
    }
}

/// The DataForge Entry of an Archive cannot be found or read.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DataForgeLoadError {
    /// The Archive holds neither `Data\Game2.dcb` nor `Data\Game.dcb`.
    #[error("the Archive holds no DataForge: neither Data\\Game2.dcb nor Data\\Game.dcb")]
    Missing,
    /// The Entry is larger than [`MAX_DATAFORGE_SIZE`] or than the memory that can be reserved.
    #[error("the DataForge Entry {path} claims {size} bytes, more than the limit of {limit} bytes")]
    TooLarge {
        /// The Entry path.
        path: String,
        /// The uncompressed size the Archive gives.
        size: u64,
        /// The largest size that is read.
        limit: u64,
    },
    /// The Entry cannot be opened in the Archive.
    #[error(transparent)]
    Archive(ArchiveError),
    /// Reading, decrypting or decompressing the Entry failed.
    #[error("cannot read the DataForge Entry {path}")]
    Read {
        /// The Entry path.
        path: String,
        /// The failed read.
        #[source]
        source: io::Error,
    },
    /// The Entry was read but holds no DataForge that can be opened.
    #[error("the DataForge cannot be opened")]
    Corrupt(#[source] DataForgeError),
    /// The Archive changed on disk while its DataForge was read.
    #[error(transparent)]
    ArchiveChanged(ArchiveChanged),
    /// No thread could be started to hold the DataForge.
    #[error("cannot start the DataForge thread")]
    Thread(#[source] io::Error),
}

impl DataForgeLoadError {
    /// Returns whether reading again could succeed: the failure came from the disk, a changed
    /// file or a missing thread, not from the content of the Entry.
    #[must_use]
    pub fn is_transient(&self) -> bool {
        match self {
            Self::Missing | Self::TooLarge { .. } | Self::Corrupt(_) => false,
            Self::Read { source, .. } => !matches!(
                source.kind(),
                io::ErrorKind::InvalidData | io::ErrorKind::UnexpectedEof
            ),
            Self::Archive(_) | Self::ArchiveChanged(_) | Self::Thread(_) => true,
        }
    }
}
fn same_path(left: &str, right: &str) -> bool {
    let key = |byte: u8| {
        if byte == b'/' {
            b'\\'
        } else {
            byte.to_ascii_lowercase()
        }
    };
    left.len() == right.len() && left.bytes().map(key).eq(right.bytes().map(key))
}
