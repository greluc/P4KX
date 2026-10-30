// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};

use anyhow::Result;
use p4kx_extraction::{Archive, DataForgeEntry, DataForgeLoadError, is_dataforge_file};

/// The DataForge a `p4kx dcb` command reads, from an Archive or from a DataForge file on disk,
/// whichever the path given is.
pub(crate) struct ForgeInput {
    path: PathBuf,
    archive: Option<Archive>,
    entry: DataForgeEntry,
}

impl ForgeInput {
    /// Reads the DataForge at `path`: the file itself when its content is a DataForge header,
    /// otherwise the DataForge Entry of the Archive at `path`.
    pub(crate) fn open(path: &Path) -> Result<Self> {
        crate::guard::on_archive_open(path);
        let sniffed = is_dataforge_file(path).map_err(|source| DataForgeLoadError::Read {
            path: path.display().to_string(),
            source,
        })?;
        if sniffed {
            let entry = DataForgeEntry::read_file(path)?;
            return Ok(Self {
                path: path.to_path_buf(),
                archive: None,
                entry,
            });
        }
        let archive = crate::key::open(path)?;
        let entry = DataForgeEntry::read(&archive)?;
        Ok(Self {
            path: path.to_path_buf(),
            archive: Some(archive),
            entry,
        })
    }

    /// Returns the path that was opened.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the Archive, or `None` when a DataForge file was opened.
    pub(crate) const fn archive(&self) -> Option<&Archive> {
        self.archive.as_ref()
    }

    /// Returns the DataForge that was read.
    pub(crate) const fn entry(&self) -> &DataForgeEntry {
        &self.entry
    }
}
