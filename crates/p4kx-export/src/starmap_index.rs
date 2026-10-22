use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use p4kx_extraction::{Archive, Entry};

pub(crate) const SYSTEM_FOLDER: &str = "data\\objectcontainers\\pu\\system\\";
pub(crate) const SOCPAK: &str = ".socpak";
const SYSTEM_SUFFIX: &str = "system.socpak";

/// The socpak Entries of an Archive by normalised path.
pub(crate) struct Index<'a> {
    pub(crate) entries: &'a [Entry],
    pub(crate) by_path: HashMap<String, usize>,
}

impl<'a> Index<'a> {
    pub(crate) fn new(archive: &'a Archive) -> Self {
        let entries = archive.entries();
        let by_path = entries
            .iter()
            .enumerate()
            .map(|(at, entry)| (key(entry.path()), at))
            .filter(|(path, _)| path.ends_with(SOCPAK))
            .collect();
        Self { entries, by_path }
    }

    pub(crate) fn find(&self, path: &str) -> Option<usize> {
        let mut wanted = key(path);
        if !wanted.starts_with("data\\") {
            wanted.insert_str(0, "data\\");
        }
        self.by_path.get(&wanted).copied()
    }

    /// Returns the folder name and the Entry of the main container of every star system, ordered
    /// by folder name: `<folder>system.socpak` of a folder below `ObjectContainers\PU\system`,
    /// else the first `*system.socpak` of the folder by path.
    pub(crate) fn systems(&self) -> Vec<(Arc<str>, usize)> {
        let mut folders: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
        for (path, at) in &self.by_path {
            let Some(rest) = path.strip_prefix(SYSTEM_FOLDER) else {
                continue;
            };
            if let Some((folder, file)) = rest.split_once('\\')
                && !file.contains('\\')
                && file.ends_with(SYSTEM_SUFFIX)
            {
                folders.entry(folder).or_default().insert(file, *at);
            }
        }
        folders
            .into_iter()
            .filter_map(|(folder, files)| {
                let own = format!("{folder}{SYSTEM_SUFFIX}");
                let at = files
                    .get(own.as_str())
                    .or_else(|| files.values().next())
                    .copied()?;
                Some((Arc::from(folder), at))
            })
            .collect()
    }
}

/// Returns `path` in lower case with `\` as separator and no leading separator.
pub(crate) fn key(path: &str) -> String {
    path.trim_start_matches(['/', '\\'])
        .chars()
        .map(|c| {
            if c == '/' {
                '\\'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect()
}
