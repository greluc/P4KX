// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};

use p4kx_extraction::{Archive, Build, DataForgeSource};
use thiserror::Error;

use super::EntryIndex;

/// A job that reads Entries was asked of a DataForge file, which has none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("a DataForge file has no Entries")]
pub struct NoEntries;

/// What an [`EntryIndex`](super::EntryIndex) stands for.
#[derive(Debug)]
pub(super) enum Backing {
    /// An Archive, or a container opened from one, with its Entries.
    Archive(Box<Archive>),
    /// A DataForge file on disk, which has no Entries.
    DataForgeFile {
        /// The file.
        path: PathBuf,
        /// Its size when it was opened.
        len: u64,
    },
}

impl EntryIndex {
    /// Returns whether this index stands for a DataForge file instead of an Archive.
    #[must_use]
    pub const fn is_dataforge_file(&self) -> bool {
        self.backing.is_dataforge_file()
    }

    /// Returns the Archive, or `None` for a DataForge file, which has no Entries.
    #[must_use]
    pub fn archive(&self) -> Option<&Archive> {
        self.backing.archive()
    }

    /// Returns the Archive for a job that reads Entries.
    ///
    /// # Errors
    ///
    /// Returns [`NoEntries`] for a DataForge file.
    pub fn entries_archive(&self) -> Result<&Archive, NoEntries> {
        self.archive().ok_or(NoEntries)
    }

    /// Returns the Build of the Archive, or `None` when it is unknown or the index stands for a
    /// DataForge file.
    #[must_use]
    pub fn build_of_archive(&self) -> Option<&Build> {
        self.archive().and_then(Archive::build)
    }

    /// Returns the path of the Archive file or DataForge file.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.backing.path()
    }

    /// Returns where the DataForge of this index is read from.
    pub(crate) fn forge_source(&self) -> DataForgeSource<'_> {
        self.backing.forge_source()
    }
}

impl Backing {
    pub(super) fn archive(&self) -> Option<&Archive> {
        match self {
            Self::Archive(archive) => Some(archive),
            Self::DataForgeFile { .. } => None,
        }
    }

    pub(super) fn path(&self) -> &Path {
        match self {
            Self::Archive(archive) => archive.path(),
            Self::DataForgeFile { path, .. } => path,
        }
    }

    pub(super) fn forge_source(&self) -> DataForgeSource<'_> {
        match self {
            Self::Archive(archive) => DataForgeSource::Archive(archive),
            Self::DataForgeFile { path, .. } => DataForgeSource::File(path),
        }
    }

    pub(super) const fn file_len(&self) -> Option<u64> {
        match self {
            Self::DataForgeFile { len, .. } => Some(*len),
            Self::Archive(_) => None,
        }
    }

    pub(super) const fn is_dataforge_file(&self) -> bool {
        matches!(self, Self::DataForgeFile { .. })
    }
}
