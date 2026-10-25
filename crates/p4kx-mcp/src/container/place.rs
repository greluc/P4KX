// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_extraction::{Archive, Entry, EntryTree, PathIndex, PathKeys};

use super::{Inner, SEPARATOR};
use crate::archives::Opened;
use crate::error::ToolError;

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
        self.index().find(path)
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
                self.containers().open(place.archive(), entry, &prefix)?
            };
            place = Place::Inner(opened);
            inner = next;
        }
        Ok((place, inner))
    }
}
