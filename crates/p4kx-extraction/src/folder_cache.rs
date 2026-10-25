// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::ExtractionError;
use crate::write::create_folders;

/// The folders below one target that a run created or checked, shared by its worker threads so
/// each folder costs its file system calls once instead of once per file.
#[derive(Debug, Default)]
pub(crate) struct FolderCache {
    known: RwLock<HashSet<PathBuf>>,
}

impl FolderCache {
    /// Returns the folder `relative` below `target`, creating it and refusing links on the way
    /// unless an earlier call did already.
    pub(crate) fn folder(
        &self,
        target: &Path,
        relative: Option<&Path>,
    ) -> Result<PathBuf, ExtractionError> {
        if let Some(relative) = relative
            && self
                .known
                .read()
                .is_ok_and(|known| known.contains(relative))
        {
            return Ok(target.join(relative));
        }
        let folder = create_folders(target, relative)?;
        if let Some(relative) = relative
            && let Ok(mut known) = self.known.write()
        {
            known.insert(relative.to_path_buf());
        }
        Ok(folder)
    }
}
