use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

const APP_FOLDER: &str = "P4K-Extractor";
const FILE_NAME: &str = "settings.toml";

/// The persistent settings of P4K-Extractor.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The manually added Channel sources.
    pub channels: ManualChannels,
}

/// Manually added folders and Archives, searched in addition to the detected Channels.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ManualChannels {
    /// Channel folders or library folders.
    pub folders: Vec<PathBuf>,
    /// Archive files.
    pub archives: Vec<PathBuf>,
}

/// A failure to read or write the settings file.
#[derive(Debug, Error)]
pub enum SettingsError {
    /// The file or its folder could not be read or written.
    #[error("cannot access the settings file {path}")]
    Io {
        /// Path of the settings file.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// The file is not valid settings TOML.
    #[error("the settings file {path} is malformed")]
    Malformed {
        /// Path of the settings file.
        path: PathBuf,
        /// What the TOML parser reported.
        #[source]
        source: toml::de::Error,
    },
    /// The settings could not be written as TOML.
    #[error("cannot write the settings as TOML")]
    Serialize(#[from] toml::ser::Error),
}

/// Returns `settings.toml` in the `P4K-Extractor` folder of the platform config directory,
/// `%APPDATA%\P4K-Extractor` on Windows.
#[must_use]
pub fn default_settings_file() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join(APP_FOLDER).join(FILE_NAME))
}

impl Settings {
    /// Reads the settings from `file`, or returns the defaults when it does not exist.
    ///
    /// # Errors
    ///
    /// Returns a [`SettingsError`] when the file cannot be read or is malformed.
    pub fn load(file: &Path) -> Result<Self, SettingsError> {
        let text = match std::fs::read_to_string(file) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(source) => return Err(io_error(file, source)),
        };
        toml::from_str(&text).map_err(|source| SettingsError::Malformed {
            path: file.to_path_buf(),
            source,
        })
    }

    /// Writes the settings to `file`, creating its folder when missing.
    ///
    /// # Errors
    ///
    /// Returns a [`SettingsError`] when the folder or the file cannot be written.
    pub fn save(&self, file: &Path) -> Result<(), SettingsError> {
        let text = toml::to_string(self)?;
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder).map_err(|source| io_error(file, source))?;
        }
        std::fs::write(file, text).map_err(|source| io_error(file, source))
    }
}

impl ManualChannels {
    /// Adds `folder` unless it is already listed; returns whether it was added.
    pub fn add_folder(&mut self, folder: PathBuf) -> bool {
        add(&mut self.folders, folder)
    }

    /// Adds `archive` unless it is already listed; returns whether it was added.
    pub fn add_archive(&mut self, archive: PathBuf) -> bool {
        add(&mut self.archives, archive)
    }

    /// Removes `path` from the folders and the Archives, matching equal or equivalent paths;
    /// returns whether it was listed.
    pub fn remove(&mut self, path: &Path) -> bool {
        let before = self.folders.len() + self.archives.len();
        self.folders.retain(|folder| !same_path(folder, path));
        self.archives.retain(|archive| !same_path(archive, path));
        before != self.folders.len() + self.archives.len()
    }
}

fn add(paths: &mut Vec<PathBuf>, path: PathBuf) -> bool {
    if paths.contains(&path) {
        return false;
    }
    paths.push(path);
    true
}

fn same_path(stored: &Path, path: &Path) -> bool {
    stored == path
        || matches!(
            (std::fs::canonicalize(stored), std::fs::canonicalize(path)),
            (Ok(a), Ok(b)) if a == b
        )
}

fn io_error(path: &Path, source: io::Error) -> SettingsError {
    SettingsError::Io {
        path: path.to_path_buf(),
        source,
    }
}
