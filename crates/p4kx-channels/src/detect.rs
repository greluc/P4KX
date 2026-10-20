use std::path::{Path, PathBuf};

use crate::{Channel, ChannelSource, library_folders};

const ARCHIVE_NAME: &str = "Data.p4k";
const LAUNCHER_FOLDER: &str = "rsilauncher";
const LAUNCHER_LOGS: &str = "logs";
const DEFAULT_LIBRARY: Option<&str> = if cfg!(windows) {
    Some(r"C:\Program Files\Roberts Space Industries\StarCitizen")
} else {
    None
};

/// The places [`detect`] looks for Channels.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchPaths {
    /// The default library folder, `C:\Program Files\Roberts Space Industries\StarCitizen` on
    /// Windows.
    pub default_library: Option<PathBuf>,
    /// The RSI Launcher's application-data folder, whose `logs` folder names library folders.
    pub launcher: Option<PathBuf>,
    /// Manually added folders, each a Channel folder or a library folder.
    pub manual_folders: Vec<PathBuf>,
    /// Manually added Archives.
    pub manual_archives: Vec<PathBuf>,
}

impl SearchPaths {
    /// Returns the default library folder and the launcher folder of this platform, without
    /// manual entries.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            default_library: DEFAULT_LIBRARY.map(PathBuf::from),
            launcher: dirs::config_dir().map(|dir| dir.join(LAUNCHER_FOLDER)),
            manual_folders: Vec::new(),
            manual_archives: Vec::new(),
        }
    }
}

/// Returns the Channels found in `paths`, each Archive once, in the order default library,
/// launcher library folders, manual folders, manual Archives.
///
/// A Channel folder is a folder holding a `Data.p4k`; a library folder holds Channel folders.
/// Only directory listings, `build_manifest.id` files and the launcher's log files are read.
#[must_use]
pub fn detect(paths: &SearchPaths) -> Vec<Channel> {
    let mut found = Found::default();
    if let Some(library) = &paths.default_library {
        found.library(library, ChannelSource::DefaultPath);
    }
    if let Some(launcher) = &paths.launcher {
        for library in library_folders(&launcher.join(LAUNCHER_LOGS)) {
            found.library(&library, ChannelSource::Launcher);
        }
    }
    for folder in &paths.manual_folders {
        let archive = folder.join(ARCHIVE_NAME);
        if archive.is_file() {
            found.archive(archive, ChannelSource::Manual);
        } else {
            found.library(folder, ChannelSource::Manual);
        }
    }
    for archive in &paths.manual_archives {
        if archive.is_file() {
            found.archive(archive.clone(), ChannelSource::Manual);
        }
    }
    found.channels
}

/// Returns the Library folders known from `paths`: the default one and those the RSI Launcher
/// log names, whether or not they exist.
///
/// Only the launcher's log files are read.
#[must_use]
pub fn known_library_folders(paths: &SearchPaths) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = paths.default_library.iter().cloned().collect();
    if let Some(launcher) = &paths.launcher {
        for library in library_folders(&launcher.join(LAUNCHER_LOGS)) {
            if !folders.contains(&library) {
                folders.push(library);
            }
        }
    }
    folders
}

#[derive(Default)]
struct Found {
    channels: Vec<Channel>,
    keys: Vec<PathBuf>,
}

impl Found {
    fn library(&mut self, library: &Path, source: ChannelSource) {
        let Ok(items) = std::fs::read_dir(library) else {
            return;
        };
        let mut archives: Vec<PathBuf> = items
            .filter_map(Result::ok)
            .map(|item| item.path().join(ARCHIVE_NAME))
            .filter(|archive| archive.is_file())
            .collect();
        archives.sort();
        for archive in archives {
            self.archive(archive, source);
        }
    }

    fn archive(&mut self, archive: PathBuf, source: ChannelSource) {
        let key = std::fs::canonicalize(&archive).unwrap_or_else(|_| archive.clone());
        if !self.keys.contains(&key) {
            self.keys.push(key);
            self.channels.push(Channel::new(archive, source));
        }
    }
}
