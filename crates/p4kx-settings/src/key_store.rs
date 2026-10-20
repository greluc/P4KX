use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use p4kx_foundation::{ArchiveKey, ArchiveKeyError};
use thiserror::Error;

const APP_FOLDER: &str = "P4K-Extractor";
const FILE_NAME: &str = "archive.key";
const MAX_KEY_FILE_LEN: u64 = 1024;

/// The environment variable that holds the Archive key.
pub const KEY_VARIABLE: &str = "P4KX_ARCHIVE_KEY";

/// A failure to read, write or remove the stored Archive key; it never contains the key.
#[derive(Debug, Error)]
pub enum KeyStoreError {
    /// The key file or its folder could not be accessed.
    #[error("cannot access the Archive key file {path}")]
    Io {
        /// Path of the key file.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// The key file does not hold an Archive key.
    #[error("the Archive key file {path} is malformed")]
    Malformed {
        /// Path of the key file.
        path: PathBuf,
        /// Why the content is not a key.
        #[source]
        source: ArchiveKeyError,
    },
}

/// The Archive key stored in the user's profile, one small file of its own next to the settings
/// file in the platform config directory, never in the repository or next to an Archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyStore {
    file: PathBuf,
}

impl KeyStore {
    /// Creates a store that keeps the key in `file`.
    #[must_use]
    pub fn new(file: impl Into<PathBuf>) -> Self {
        Self { file: file.into() }
    }

    /// Returns the store in the `P4K-Extractor` folder of the platform config directory,
    /// `%APPDATA%\P4K-Extractor\archive.key` on Windows, or `None` when the platform has no
    /// config directory.
    #[must_use]
    pub fn in_profile() -> Option<Self> {
        dirs::config_dir().map(|dir| Self::new(dir.join(APP_FOLDER).join(FILE_NAME)))
    }

    /// Returns the path of the key file.
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Reads the stored key, or `None` when none is stored.
    ///
    /// # Errors
    ///
    /// Returns a [`KeyStoreError`] when the file cannot be read or does not hold a key.
    pub fn load(&self) -> Result<Option<ArchiveKey>, KeyStoreError> {
        let opened = match std::fs::File::open(&self.file) {
            Ok(opened) => opened,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(self.io_error(source)),
        };
        let mut text = String::new();
        opened
            .take(MAX_KEY_FILE_LEN)
            .read_to_string(&mut text)
            .map_err(|source| self.io_error(source))?;
        ArchiveKey::from_hex(&text)
            .map(Some)
            .map_err(|source| KeyStoreError::Malformed {
                path: self.file.clone(),
                source,
            })
    }

    /// Stores `key`, creating the folder when missing. The text goes to a temporary file next to
    /// the key file first, readable by the user alone where the platform supports it, which then
    /// replaces the key file, so a crash never leaves a half-written file.
    ///
    /// On Windows the file keeps the permissions its folder in the user's profile gives it.
    ///
    /// # Errors
    ///
    /// Returns a [`KeyStoreError`] when the folder or the file cannot be written.
    pub fn save(&self, key: &ArchiveKey) -> Result<(), KeyStoreError> {
        if let Some(folder) = self.file.parent() {
            std::fs::create_dir_all(folder).map_err(|source| self.io_error(source))?;
        }
        let mut temporary = self.file.as_os_str().to_owned();
        temporary.push(format!(".{}.tmp", std::process::id()));
        let temporary = PathBuf::from(temporary);
        let mut text = key.to_hex();
        text.push('\n');
        let written = write_private(&temporary, text.as_bytes())
            .and_then(|()| std::fs::rename(&temporary, &self.file));
        if let Err(source) = written {
            let _ignored = std::fs::remove_file(&temporary);
            return Err(self.io_error(source));
        }
        Ok(())
    }

    /// Removes the stored key; returns whether one was stored.
    ///
    /// # Errors
    ///
    /// Returns a [`KeyStoreError`] when the file exists but cannot be removed.
    pub fn remove(&self) -> Result<bool, KeyStoreError> {
        match std::fs::remove_file(&self.file) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(source) => Err(self.io_error(source)),
        }
    }

    fn io_error(&self, source: io::Error) -> KeyStoreError {
        KeyStoreError::Io {
            path: self.file.clone(),
            source,
        }
    }
}

#[cfg(unix)]
fn create_private(path: &Path) -> io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;

    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(unix))]
fn create_private(path: &Path) -> io::Result<std::fs::File> {
    std::fs::File::create(path)
}

fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = create_private(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}
