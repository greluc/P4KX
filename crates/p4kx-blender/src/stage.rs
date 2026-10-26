// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process;

use serde::Deserialize;

use crate::manifest::{FORMAT, MANIFEST_FILE, MESHES_FOLDER, TEXTURES_FOLDER};
use crate::rename::{is_busy, rename};
use crate::{PackageError, PackageResult};

const MAX_MARKER_LEN: u64 = 1 << 30;
const MIN_ID_DIGITS: usize = 4;

#[derive(Deserialize)]
struct Marker {
    format: String,
}

/// Checks that a package may be written to `out`, before any work is done for it.
///
/// `out` must lie outside every Star Citizen installation, and must not exist, or be an empty
/// folder, or hold only a package an earlier run wrote: `scene.json` with the format name, a
/// `meshes` folder of `m<digits>.glb` files and a `textures` folder of `t<digits>.png` files.
///
/// # Errors
///
/// Returns [`PackageError::Refused`] inside an installation, [`PackageError::NotAPackage`] for
/// anything else that exists, [`PackageError::NoFolderName`] when `out` names no folder and
/// [`PackageError::Write`] when `out` cannot be inspected.
pub fn check_output(out: &Path) -> PackageResult<()> {
    p4kx_guard::ensure_writable(out)?;
    if out.file_name().is_none() {
        return Err(PackageError::NoFolderName {
            path: out.to_path_buf(),
        });
    }
    check_replaceable(out)
}

/// A temporary folder next to the output that becomes the output once the package is complete and
/// is removed when it is dropped before.
#[derive(Debug)]
pub(crate) struct Staging {
    folder: PathBuf,
    out: PathBuf,
    created: Vec<PathBuf>,
    committed: bool,
}

impl Staging {
    /// Checks that `out` may be written and creates the temporary folder next to it; parent
    /// folders that did not exist are created and are removed again when the staging is dropped
    /// before it is committed.
    pub(crate) fn create(out: &Path) -> PackageResult<Self> {
        check_output(out)?;
        let name = out.file_name().map(OsStr::to_os_string).unwrap_or_default();
        let parent = parent_of(out);
        let created = missing_ancestors(&parent);
        if !parent.as_os_str().is_empty()
            && let Err(source) = fs::create_dir_all(&parent)
        {
            remove_empty(&created);
            return Err(PackageError::write(&parent, source));
        }
        let folder = parent.join(sibling_name(&name, "part"));
        if let Err(source) = fs::create_dir(&folder) {
            remove_empty(&created);
            return Err(PackageError::write(&folder, source));
        }
        let staging = Self {
            folder,
            out: out.to_path_buf(),
            created,
            committed: false,
        };
        for sub in [MESHES_FOLDER, TEXTURES_FOLDER] {
            let path = staging.folder.join(sub);
            fs::create_dir(&path).map_err(|source| PackageError::write(&path, source))?;
        }
        Ok(staging)
    }

    /// Returns the temporary folder.
    pub(crate) fn path(&self) -> &Path {
        &self.folder
    }

    /// Flushes the temporary folder to disk and moves it to the output, replacing an earlier
    /// package; the earlier package stays when the move fails. A move that fails because another
    /// program holds a folder open is tried again for a short while first.
    pub(crate) fn commit(mut self) -> PackageResult<()> {
        let out = self.out.clone();
        check_output(&out)?;
        for path in [
            self.folder.join(MESHES_FOLDER),
            self.folder.join(TEXTURES_FOLDER),
            self.folder.clone(),
        ] {
            sync_folder(&path).map_err(|source| PackageError::write(&path, source))?;
        }
        let name = out.file_name().map(OsStr::to_os_string).unwrap_or_default();
        let old = parent_of(&out).join(sibling_name(&name, "old"));
        let replaced = fs::symlink_metadata(&out).is_ok();
        if replaced {
            rename(&out, &old).map_err(|source| moving_error(&out, source))?;
        }
        if let Err(source) = rename(&self.folder, &out) {
            if replaced && let Err(restore) = rename(&old, &out) {
                return Err(PackageError::NotRestored {
                    path: out,
                    old,
                    source: restore,
                });
            }
            return Err(moving_error(&self.folder, source));
        }
        self.committed = true;
        if replaced {
            let _ignored = fs::remove_dir_all(&old);
        }
        let _ignored = sync_folder(&parent_of(&out));
        Ok(())
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if !self.committed {
            let _ignored = fs::remove_dir_all(&self.folder);
            remove_empty(&self.created);
        }
    }
}

fn moving_error(path: &Path, source: io::Error) -> PackageError {
    if is_busy(&source) {
        PackageError::InUse {
            path: path.to_path_buf(),
            source,
        }
    } else {
        PackageError::write(path, source)
    }
}

/// Returns the folders of `path` and above that do not exist yet, the deepest first.
fn missing_ancestors(path: &Path) -> Vec<PathBuf> {
    path.ancestors()
        .take_while(|ancestor| {
            !ancestor.as_os_str().is_empty() && fs::symlink_metadata(ancestor).is_err()
        })
        .map(Path::to_path_buf)
        .collect()
}

fn remove_empty(folders: &[PathBuf]) {
    for folder in folders {
        let _ignored = fs::remove_dir(folder);
    }
}

#[cfg(unix)]
fn sync_folder(folder: &Path) -> io::Result<()> {
    File::open(folder)?.sync_all()
}

#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "same signature as the Unix version; Windows cannot flush a folder"
)]
const fn sync_folder(_folder: &Path) -> io::Result<()> {
    Ok(())
}

fn parent_of(out: &Path) -> PathBuf {
    out.parent().map(Path::to_path_buf).unwrap_or_default()
}

fn sibling_name(name: &OsString, kind: &str) -> OsString {
    let mut sibling = name.clone();
    sibling.push(format!(".p4kx-{}.{kind}", process::id()));
    sibling
}

fn check_replaceable(out: &Path) -> PackageResult<()> {
    let not_a_package = || PackageError::NotAPackage {
        path: out.to_path_buf(),
    };
    let io_error = |path: &Path, source| PackageError::write(path, source);
    let metadata = match fs::symlink_metadata(out) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(io_error(out, source)),
    };
    if !metadata.is_dir() {
        return Err(not_a_package());
    }
    let names = names_in(out).map_err(|source| io_error(out, source))?;
    if names.is_empty() {
        return Ok(());
    }
    let known = [MANIFEST_FILE, MESHES_FOLDER, TEXTURES_FOLDER];
    if !names
        .iter()
        .all(|name| known.iter().any(|known| name == known))
        || !is_manifest(&out.join(MANIFEST_FILE))
    {
        return Err(not_a_package());
    }
    for (folder, prefix, extension) in [(MESHES_FOLDER, 'm', "glb"), (TEXTURES_FOLDER, 't', "png")]
    {
        let path = out.join(folder);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => return Err(not_a_package()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(source) => return Err(io_error(&path, source)),
        }
        let files = names_in(&path).map_err(|source| io_error(&path, source))?;
        if !files
            .iter()
            .all(|name| is_id_file(name, prefix, extension) && is_plain_file(&path.join(name)))
        {
            return Err(not_a_package());
        }
    }
    Ok(())
}

fn names_in(folder: &Path) -> io::Result<Vec<OsString>> {
    fs::read_dir(folder)?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect()
}

fn is_plain_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file())
}

/// Returns whether `name` is `<prefix><at least four digits>.<extension>`.
fn is_id_file(name: &OsStr, prefix: char, extension: &str) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    let Some(stem) = name
        .strip_suffix(extension)
        .and_then(|rest| rest.strip_suffix('.'))
        .and_then(|rest| rest.strip_prefix(prefix))
    else {
        return false;
    };
    stem.len() >= MIN_ID_DIGITS && stem.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_manifest(path: &Path) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    let reader = BufReader::new(file.take(MAX_MARKER_LEN));
    serde_json::from_reader::<_, Marker>(reader).is_ok_and(|marker| marker.format == FORMAT)
}
