// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use p4kx_extraction::Entry;

/// Lists every file under `root`, depth first, with symbolic links ignored.
pub(crate) fn files_under(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for item in std::fs::read_dir(&folder)? {
            let item = item?;
            let kind = item.file_type()?;
            if kind.is_dir() {
                folders.push(item.path());
            } else if kind.is_file() {
                files.push(item.path());
            }
        }
    }
    files.sort();
    Ok(files)
}

/// The key an Entry path and a path in a reference folder share: lower case, backslashes.
#[must_use]
pub(crate) fn entry_key(path: &str) -> String {
    path.replace('/', "\\")
        .trim_start_matches('\\')
        .to_lowercase()
}

/// Returns the key of `file` relative to `root`.
#[must_use]
pub(crate) fn relative_key(root: &Path, file: &Path) -> Option<String> {
    file.strip_prefix(root)
        .ok()
        .map(|relative| entry_key(&relative.to_string_lossy()))
}

/// Indexes the Entries by key; of several Entries with one path the last wins, as it does when
/// the tools extract them in directory order.
#[must_use]
pub(crate) fn index_entries(entries: &[Entry]) -> HashMap<String, &Entry> {
    entries
        .iter()
        .map(|entry| (entry_key(entry.path()), entry))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{entry_key, relative_key};
    use std::path::Path;

    #[test]
    fn a_reference_path_and_an_entry_path_share_one_key() {
        let root = Path::new("ref");
        let file = Path::new("ref").join("Data").join("Libs").join("A.xml");
        assert_eq!(
            relative_key(root, &file),
            Some(entry_key("Data\\libs\\a.XML")),
            "the same Entry"
        );
        assert_eq!(entry_key("/Data/a"), "data\\a", "slashes and a leading one");
    }
}
