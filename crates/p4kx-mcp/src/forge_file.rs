// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use p4kx_extraction::{DataForge, DataForgeSource};

use crate::archives::Opened;
use crate::error::ToolError;
use crate::forge_state::ForgeState;

/// What identifies a version of a file on disk: its size and modification time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileVersion {
    len: u64,
    modified: Option<SystemTime>,
}

impl FileVersion {
    /// Returns the version of the file at `path` as it is now, or the empty version when it
    /// cannot be looked at.
    pub(crate) fn of(path: &Path) -> Self {
        std::fs::metadata(path).map_or(
            Self {
                len: 0,
                modified: None,
            },
            |metadata| Self {
                len: metadata.len(),
                modified: metadata.modified().ok(),
            },
        )
    }
}

/// A DataForge file on disk, such as a `Game2.dcb`, with its DataForge read on first use and
/// kept while the file stays unchanged.
#[derive(Debug)]
pub(crate) struct ForgeFile {
    path: PathBuf,
    version: FileVersion,
    forge: ForgeState,
}

impl ForgeFile {
    /// Creates the handle of the file at `path`, which has `version`.
    pub(crate) fn new(path: PathBuf, version: FileVersion) -> Self {
        Self {
            path,
            version,
            forge: ForgeState::default(),
        }
    }

    /// Returns whether the file on disk is still the one this handle was made for.
    pub(crate) fn is_version(&self, version: FileVersion) -> bool {
        self.version == version
    }
}

/// The input of a DataForge tool: an opened Archive or a DataForge file.
#[derive(Debug, Clone)]
pub(crate) enum ForgeSource {
    /// The DataForge Entry of an Archive.
    Archive(Arc<Opened>),
    /// A DataForge file on disk.
    File(Arc<ForgeFile>),
}

impl ForgeSource {
    /// Returns the path, size and modification time of the input as it was opened, which bind a
    /// cursor to this version of it.
    pub(crate) fn version(&self) -> (&Path, u64, Option<SystemTime>) {
        match self {
            Self::Archive(opened) => opened.version(),
            Self::File(file) => (&file.path, file.version.len, file.version.modified),
        }
    }

    /// Runs `work` against the DataForge with the input path as text, reading the DataForge on
    /// first use.
    pub(crate) fn with_dataforge<T, F>(&self, work: F) -> Result<T, ToolError>
    where
        T: Send + 'static,
        F: FnOnce(&DataForge<'_>, String) -> Result<T, ToolError> + Send + 'static,
    {
        match self {
            Self::Archive(opened) => opened.with_dataforge(work),
            Self::File(file) => file.forge.with(
                || DataForgeSource::File(&file.path).load(),
                file.path.display().to_string(),
                work,
            ),
        }
    }

    /// Reads the DataForge if that has not happened yet, without running any work on it.
    pub(crate) fn load_dataforge(&self) -> Result<(), ToolError> {
        match self {
            Self::Archive(opened) => opened.load_dataforge(),
            Self::File(file) => file.forge.load(|| DataForgeSource::File(&file.path).load()),
        }
    }

    /// Returns the selection of the last Query run if it was of `query`.
    pub(crate) fn selection(&self, query: &str) -> Option<Arc<[u32]>> {
        match self {
            Self::Archive(opened) => opened.selection(query),
            Self::File(file) => file.forge.selection(query),
        }
    }

    /// Keeps `records` as the selection of the last Query run, which was of `query`.
    pub(crate) fn keep_selection(&self, query: &str, records: Arc<[u32]>) {
        match self {
            Self::Archive(opened) => opened.keep_selection(query, records),
            Self::File(file) => file.forge.keep_selection(query, records),
        }
    }
}
