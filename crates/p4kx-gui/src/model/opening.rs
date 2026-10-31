// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::Path;

use p4kx_extraction::{Archive, ArchiveError, ArchiveKey, is_dataforge_file};

use crate::EntryIndex;

/// Opens what is at `path`: a DataForge file, recognised by its content, or else an Archive
/// opened with `key`.
pub(super) fn open_index(path: &Path, key: Option<ArchiveKey>) -> Result<EntryIndex, ArchiveError> {
    p4kx_extraction::protect_archive_folder(path);
    if is_dataforge_file(path).unwrap_or(false) {
        return EntryIndex::build_dataforge_file(path.to_path_buf()).map_err(|source| {
            ArchiveError::Io {
                path: path.to_path_buf(),
                source,
            }
        });
    }
    Archive::open_with_key(path, key).map(EntryIndex::build)
}
