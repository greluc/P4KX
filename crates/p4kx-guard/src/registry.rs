use std::path::Path;
use std::sync::{PoisonError, RwLock};

use p4kx_channels::{SearchPaths, known_library_folders};

use crate::{Installation, WriteRefused};

const GAME_FOLDER: &str = "Bin64";

static PROTECTED: RwLock<Installation> = RwLock::new(Installation::new());

/// Protects `folder` and everything below it for the rest of the process.
pub fn protect(folder: &Path) {
    PROTECTED
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .add(folder);
}

/// Protects the Library folders of this machine and, when it holds a game installation, the
/// folder of the Archive at `archive`.
///
/// The Library folders are the default one and those the RSI Launcher log names. The Archive's
/// folder is a game installation when it holds the game's `Bin64` folder; a copy of an Archive
/// elsewhere is not one.
pub fn protect_archive_folder(archive: &Path) {
    for library in known_library_folders(&SearchPaths::standard()) {
        protect(&library);
    }
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
