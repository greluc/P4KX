// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};
use std::sync::{Once, PoisonError, RwLock};

use p4kx_channels::{SearchPaths, known_library_folders};

use crate::{Installation, WriteRefused};

const GAME_FOLDER: &str = "Bin64";
const EXTRA_FOLDERS_VARIABLE: &str = "P4KX_PROTECT_FOLDERS";

static PROTECTED: RwLock<Installation> = RwLock::new(Installation::new());
static LIBRARIES: Once = Once::new();

/// Protects `folder` and everything below it for the rest of the process.
pub fn protect(folder: &Path) {
    PROTECTED
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .add(folder);
}

/// Protects the Library folders of this machine: the default one and those the RSI Launcher log
/// names. Call it once at program start, before any file is written.
///
/// The folders in the path list of the environment variable `P4KX_PROTECT_FOLDERS` are protected
/// as well. It is a test aid for fake installations and can only add protection.
pub fn protect_libraries() {
    LIBRARIES.call_once(|| {
        for library in known_library_folders(&SearchPaths::standard()) {
            protect(&library);
        }
    });
    if let Some(extra) = std::env::var_os(EXTRA_FOLDERS_VARIABLE) {
        for folder in std::env::split_paths(&extra).filter(|folder| !folder.as_os_str().is_empty())
        {
            protect(&folder);
        }
    }
}

/// Protects the Library folders of this machine and, when it holds a game installation, the
/// folder of the Archive at `archive`.
///
/// The Archive's folder is a game installation when it holds the game's `Bin64` folder; a copy of
/// an Archive elsewhere is not one. The launcher logs are read once per process.
pub fn protect_archive_folder(archive: &Path) {
    protect_libraries();
    protect_installation_of(archive);
}

/// Protects the folders the user entered by hand as Library or Channel folders, and the folder of
/// each Archive entered by hand that holds the game's `Bin64` folder.
///
/// Call it with the manual entries of the settings whenever they are loaded or extended.
pub fn protect_manual(folders: &[PathBuf], archives: &[PathBuf]) {
    for folder in folders {
        protect(folder);
    }
    for archive in archives {
        protect_installation_of(archive);
    }
}

fn protect_installation_of(archive: &Path) {
    if let Some(folder) = std::path::absolute(archive)
        .ok()
        .and_then(|absolute| absolute.parent().map(Path::to_path_buf))
        && folder.join(GAME_FOLDER).is_dir()
    {
        protect(&folder);
    }
}

/// Checks that writing to `target` stays outside the protected folders.
///
/// # Errors
///
/// Returns [`WriteRefused`] when `target` lies inside a protected folder.
pub fn ensure_writable(target: &Path) -> Result<(), WriteRefused> {
    PROTECTED
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .check(target)
}
