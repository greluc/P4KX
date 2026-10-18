use std::io::{self, Read};

use p4kx_archive::{Archive, ArchiveError, Entry};
use p4kx_dataforge::{DataForge, DataForgeError};
use thiserror::Error;

/// The Entries that may hold DataForge, the preferred one first.
pub const DATAFORGE_ENTRIES: [&str; 2] = ["Data\\Game2.dcb", "Data\\Game.dcb"];

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
    /// Returns [`DataForgeLoadError::Missing`] when the Archive holds neither Entry, and another
    /// [`DataForgeLoadError`] when the Entry cannot be read.
    pub fn read(archive: &Archive) -> Result<Self, DataForgeLoadError> {
        let entry = find_entry(archive).ok_or(DataForgeLoadError::Missing)?;
        let path = entry.path().to_owned();
        let size = usize::try_from(entry.uncompressed_size()).map_err(|_| {
            DataForgeLoadError::TooLarge {
                path: path.clone(),
                size: entry.uncompressed_size(),
            }
        })?;
        let mut bytes = Vec::new();
        if bytes.try_reserve_exact(size).is_err() {
            return Err(DataForgeLoadError::TooLarge {
                path,
                size: entry.uncompressed_size(),
            });
        }
        let read = archive
            .open_entry(entry)
            .map_err(DataForgeLoadError::Archive)
            .and_then(|mut reader| {
                reader
                    .read_to_end(&mut bytes)
                    .map_err(|source| DataForgeLoadError::Read {
                        path: path.clone(),
                        source,
                    })
            });
        read.map(|_| Self { path, bytes })
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
    /// The Entry is larger than the memory that can be reserved for it.
    #[error("the DataForge Entry {path} of {size} bytes does not fit into memory")]
    TooLarge {
        /// The Entry path.
        path: String,
        /// The uncompressed size the Archive gives.
        size: u64,
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
}

fn find_entry(archive: &Archive) -> Option<&Entry> {
    DATAFORGE_ENTRIES.iter().find_map(|wanted| {
        archive
            .entries()
            .iter()
            .find(|entry| same_path(entry.path(), wanted))
    })
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
