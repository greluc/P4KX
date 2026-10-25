// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use p4kx_extraction::{Archive, Entry, EntryTree, PathIndex, PathKeys, find_entry};

use crate::archives::Opened;
use crate::error::ToolError;

/// The text between a container and a path inside it, as in `outer.socpak::inner.socpak::file`.
pub(crate) const SEPARATOR: &str = "::";

/// How many opened containers stay in memory per Archive.
const CAPACITY: usize = 4;

/// The containers opened from one Archive, most recently used first.
#[derive(Debug, Default)]
pub(crate) struct Containers {
    kept: Mutex<Vec<Arc<Inner>>>,
}

/// A container opened as an Archive of its own with what is built from it once.
#[derive(Debug)]
pub(crate) struct Inner {
    prefix: String,
    archive: Archive,
    tree: OnceLock<EntryTree>,
    keys: OnceLock<PathKeys>,
}

impl Containers {
    fn open(
        &self,
        parent: &Archive,
        entry: &Entry,
        prefix: String,
    ) -> Result<Arc<Inner>, ToolError> {
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(index) = kept.iter().position(|inner| inner.prefix == prefix) {
            let inner = kept.remove(index);
            kept.insert(0, Arc::clone(&inner));
            return Ok(inner);
        }
        let archive = parent
            .open_container(entry)
            .map_err(|source| ToolError::Container {
                path: prefix.clone(),
                source,
            })?;
        let inner = Arc::new(Inner {
            prefix,
            archive,
            tree: OnceLock::new(),
            keys: OnceLock::new(),
        });
        kept.insert(0, Arc::clone(&inner));
        kept.truncate(CAPACITY);
        drop(kept);
        Ok(inner)
    }
}

/// Where a path points: the Archive itself or a container inside it.
#[derive(Debug)]
pub(crate) enum Place<'a> {
    /// The Archive.
    Main(&'a Opened),
    /// A container, at any depth.
    Inner(Arc<Inner>),
}

impl Place<'_> {
    /// Returns the Archive or container to read Entries from.
    pub(crate) fn archive(&self) -> &Archive {
        match self {
            Self::Main(opened) => opened.archive(),
            Self::Inner(inner) => &inner.archive,
        }
    }

    /// Returns the folder tree, building it on first use.
    pub(crate) fn tree(&self) -> &EntryTree {
        match self {
            Self::Main(opened) => opened.tree(),
            Self::Inner(inner) => inner.tree.get_or_init(|| EntryTree::build(&inner.archive)),
        }
    }

    /// Returns the Entries by path, hashing the paths on first use.
    pub(crate) fn index(&self) -> PathIndex<'_> {
        match self {
            Self::Main(opened) => opened.index(),
            Self::Inner(inner) => inner
                .keys
                .get_or_init(|| PathKeys::new(&inner.archive))
                .index(&inner.archive),
        }
    }

    /// Returns `path` inside this place as a path every tool takes: prefixed with the container
    /// chain when this is a container.
    pub(crate) fn shown(&self, path: &str) -> String {
        match self {
            Self::Main(_) => path.to_owned(),
            Self::Inner(inner) => format!("{}{SEPARATOR}{path}", inner.prefix),
        }
    }

    /// Returns the container chain as the Entry paths of the containers, or `None` for the Archive.
    pub(crate) fn chain(&self) -> Option<&str> {
        match self {
            Self::Main(_) => None,
            Self::Inner(inner) => Some(&inner.prefix),
        }
    }

    fn find(&self, path: &str) -> Option<&Entry> {
        match self {
            Self::Main(opened) => opened.index().find(path),
            Self::Inner(inner) => find_entry(&inner.archive, path),
        }
    }
}

impl Opened {
    /// Resolves `path`, which may begin with containers, as in
    /// `outer.socpak::inner.socpak::folder/file.xml`: opens the containers in turn and returns the
    /// place of the last one with the rest of the path. A path without `::` is a path of the
    /// Archive itself.
    pub(crate) fn place<'s, 'p>(
        &'s self,
        path: &'p str,
    ) -> Result<(Place<'s>, &'p str), ToolError> {
        let mut parts = path.split(SEPARATOR);
        let mut inner = parts.next().unwrap_or_default();
        let mut place = Place::Main(self);
        for next in parts {
            if inner.trim().is_empty() {
                return Err(ToolError::BadContainerPath(path.to_owned()));
            }
            let opened = {
                let entry = place
                    .find(inner)
                    .ok_or_else(|| ToolError::UnknownEntry(place.shown(inner)))?;
                let prefix = place.shown(entry.path());
                self.containers().open(place.archive(), entry, prefix)?
            };
            place = Place::Inner(opened);
            inner = next;
        }
        Ok((place, inner))
    }
}

#[cfg(test)]
mod tests {
    use super::SEPARATOR;

    #[test]
    fn the_separator_splits_a_chain_into_containers_and_the_rest() {
        let parts: Vec<&str> = "a.socpak::b.socpak::c\\d.xml".split(SEPARATOR).collect();
        assert_eq!(parts, ["a.socpak", "b.socpak", "c\\d.xml"]);
        assert_eq!(
            "a.socpak::".split(SEPARATOR).collect::<Vec<_>>(),
            ["a.socpak", ""],
            "an empty rest is the container's top level"
        );
    }
}
