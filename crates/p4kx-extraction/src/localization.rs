use std::io::{self, Read};

use p4kx_archive::{Archive, ArchiveError, Entry};
use p4kx_localization::{GlobalIni, Language, LocalizationError, MAX_GLOBAL_INI_SIZE};
use rayon::prelude::*;
use thiserror::Error;

use crate::entry_path::same_entry_path;

const INITIAL_RESERVE: u64 = 16 << 20;

/// The bytes of a language's `global.ini` Entry, read into memory.
#[derive(Debug)]
pub struct LocalizationEntry {
    language: Language,
    path: String,
    bytes: Vec<u8>,
}

impl LocalizationEntry {
    /// Reads the `global.ini` of `language` from `archive`; case and separator kind of the path do
    /// not matter.
    ///
    /// # Errors
    ///
    /// Returns [`LocalizationLoadError::Missing`] when the Archive holds no such Entry,
    /// [`LocalizationLoadError::TooLarge`] when its size exceeds [`MAX_GLOBAL_INI_SIZE`], and
    /// another [`LocalizationLoadError`] when the Entry cannot be read.
    pub fn read(archive: &Archive, language: Language) -> Result<Self, LocalizationLoadError> {
        let entry =
            Self::locate(archive, language).ok_or(LocalizationLoadError::Missing { language })?;
        let path = entry.path().to_owned();
        let size = entry.uncompressed_size();
        let too_large = |path| LocalizationLoadError::TooLarge {
            path,
            size,
            limit: MAX_GLOBAL_INI_SIZE,
        };
        if size > MAX_GLOBAL_INI_SIZE {
            return Err(too_large(path));
        }
        let mut bytes = Vec::new();
        let reserve = usize::try_from(size.min(INITIAL_RESERVE)).unwrap_or_default();
        if bytes.try_reserve_exact(reserve).is_err() {
            return Err(too_large(path));
        }
        let failed = |source| LocalizationLoadError::Read {
            path: path.clone(),
            source,
        };
        let read = archive
            .open_entry(entry)
            .map_err(LocalizationLoadError::Archive)?
            .take(size)
            .read_to_end(&mut bytes)
            .map_err(failed)?;
        if u64::try_from(read).ok() != Some(size) {
            return Err(failed(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("the Entry ended after {read} of {size} bytes"),
            )));
        }
        Ok(Self {
            language,
            path,
            bytes,
        })
    }

    /// Returns the `global.ini` Entry of `language`, or `None` when the Archive holds none.
    #[must_use]
    pub fn locate(archive: &Archive, language: Language) -> Option<&Entry> {
        let wanted = language.global_ini_path();
        archive
            .entries()
            .par_iter()
            .find_first(|entry| same_entry_path(entry.path(), wanted))
    }

    /// Returns the language.
    #[must_use]
    pub const fn language(&self) -> Language {
        self.language
    }

    /// Returns the path of the Entry the file was read from.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Indexes the file's texts by localisation key.
    ///
    /// # Errors
    ///
    /// Returns a [`LocalizationError`] when the file is not UTF-8 or exceeds a limit.
    pub fn parse(&self) -> Result<GlobalIni<'_>, LocalizationError> {
        GlobalIni::parse(&self.bytes)
    }
}

/// The `global.ini` of a language cannot be found or read.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LocalizationLoadError {
    /// The Archive holds no `global.ini` for the language.
    #[error("the Archive holds no global.ini for the language {language}")]
    Missing {
        /// The language.
        language: Language,
    },
    /// The Entry is larger than [`MAX_GLOBAL_INI_SIZE`] or than the memory that can be reserved.
    #[error("the Entry {path} claims {size} bytes, more than the limit of {limit} bytes")]
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
    #[error("cannot read the Entry {path}")]
    Read {
        /// The Entry path.
        path: String,
        /// The failed read.
        #[source]
        source: io::Error,
    },
}
