// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io;
use std::path::PathBuf;
use std::sync::OnceLock;

use p4kx_channels::ChannelKind;
use p4kx_extraction::{DataForgeSource, EntryTree};

use super::EntryIndex;
use super::backing::Backing;
use crate::entry_groups::EntryGroups;
use crate::forge_cache::ForgeCache;
use crate::sort::SortOrders;
use crate::text_cache::TextCache;

impl EntryIndex {
    /// Builds the index of the DataForge file at `path`: no Entries and no Archive, and the
    /// DataForge is read from the file itself.
    ///
    /// # Errors
    ///
    /// Returns the error of looking at the file.
    pub fn build_dataforge_file(path: PathBuf) -> io::Result<Self> {
        let watch = DataForgeSource::File(&path).watch()?;
        let len = std::fs::metadata(&path)?.len();
        Ok(Self {
            channel: ChannelKind::of_archive(&path),
            backing: Backing::DataForgeFile { path, len },
            watch,
            tree: EntryTree::default(),
            orders: SortOrders::default(),
            containers: Vec::new(),
            path_bytes: 0,
            groups: EntryGroups::default(),
            id_of: Vec::new(),
            dataforge: None,
            forge: ForgeCache::default(),
            paths: OnceLock::new(),
            text_languages: Vec::new(),
            texts: TextCache::default(),
        })
    }
}
