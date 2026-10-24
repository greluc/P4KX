// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process;

use serde::Deserialize;

use crate::manifest::{FORMAT, MANIFEST_FILE, PARTS_FOLDER, TEXTURES_FOLDER};
use crate::{PackageError, PackageResult};

const MAX_MARKER_LEN: u64 = 1 << 30;

#[derive(Deserialize)]
struct Marker {
    format: String,
}

/// A temporary folder next to the output that becomes the output once the package is complete and
/// is removed when it is dropped before.
#[derive(Debug)]
pub(crate) struct Staging {
    folder: PathBuf,
    out: PathBuf,
    committed: bool,
}

impl Staging {
    /// Checks that `out` may be written and creates the temporary folder next to it.
    pub(crate) fn create(out: &Path) -> PackageResult<Self> {
        p4kx_guard::ensure_writable(out)?;
        let name = out
            .file_name()
            .ok_or_else(|| PackageError::NoFolderName {
                path: out.to_path_buf(),
            })?
            .to_os_string();
        check_replaceable(out)?;
        let parent = parent_of(out);
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(&parent).map_err(|source| PackageError::write(&parent, source))?;
        }
        let folder = parent.join(sibling_name(&name, "part"));
        fs::create_dir(&folder).map_err(|source| PackageError::write(&folder, source))?;
        let staging = Self {
            folder,
            out: out.to_path_buf(),
            committed: false,
        };
        for sub in [PARTS_FOLDER, TEXTURES_FOLDER] {
            let path = staging.folder.join(sub);
            fs::create_dir(&path).map_err(|source| PackageError::write(&path, source))?;
        }
        Ok(staging)
    }

    /// Returns the temporary folder.
    pub(crate) fn path(&self) -> &Path {
        &self.folder
    }

    /// Moves the temporary folder to the output, replacing an earlier package; the earlier
    /// package stays when the move fails.
    pub(crate) fn commit(mut self) -> PackageResult<()> {
        let out = self.out.clone();
        let name = out.file_name().map(OsString::from).unwrap_or_default();
        let old = parent_of(&out).join(sibling_name(&name, "old"));
        let replaced = fs::symlink_metadata(&out).is_ok();
        if replaced {
            fs::rename(&out, &old).map_err(|source| PackageError::write(&out, source))?;
        }
        if let Err(source) = fs::rename(&self.folder, &out) {
            if replaced {
                let _restored = fs::rename(&old, &out);
            }
            return Err(PackageError::write(&out, source));
        }
        self.committed = true;
        if replaced {
            let _ignored = fs::remove_dir_all(&old);
        }
        Ok(())
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if !self.committed {
            let _ignored = fs::remove_dir_all(&self.folder);
        }
    }
}

fn parent_of(out: &Path) -> PathBuf {
    out.parent().map(Path::to_path_buf).unwrap_or_default()
}

fn sibling_name(name: &OsString, kind: &str) -> OsString {
    let mut sibling = name.clone();
    sibling.push(format!(".p4kx-{}.{kind}", process::id()));
    sibling
}

/// Accepts an output that does not exist, an empty folder, or a folder that holds only a package
/// of this tool: a manifest with the format name, and the parts and textures folders.
fn check_replaceable(out: &Path) -> PackageResult<()> {
    let not_a_package = || PackageError::NotAPackage {
        path: out.to_path_buf(),
    };
    let metadata = match fs::symlink_metadata(out) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(PackageError::write(out, source)),
    };
    if !metadata.is_dir() {
        return Err(not_a_package());
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(out).map_err(|source| PackageError::write(out, source))? {
        let entry = entry.map_err(|source| PackageError::write(out, source))?;
        entries.push(entry.file_name());
    }
    if entries.is_empty() {
        return Ok(());
    }
    let known = [MANIFEST_FILE, PARTS_FOLDER, TEXTURES_FOLDER];
    let only_known = entries
        .iter()
        .all(|name| known.iter().any(|known| name == known));
    if only_known && is_manifest(&out.join(MANIFEST_FILE)) {
        Ok(())
    } else {
        Err(not_a_package())
    }
}

fn is_manifest(path: &Path) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    let reader = BufReader::new(file.take(MAX_MARKER_LEN));
    serde_json::from_reader::<_, Marker>(reader).is_ok_and(|marker| marker.format == FORMAT)
}
