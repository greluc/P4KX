// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::OnceLock;

use p4kx_archive::Archive;

use super::AudioIndex;

/// The audio of one open Archive: its [`AudioIndex`] is built on the first use and kept.
#[derive(Debug)]
pub struct ArchiveAudio<'a> {
    archive: &'a Archive,
    index: OnceLock<AudioIndex>,
}

impl<'a> ArchiveAudio<'a> {
    /// Reads audio from `archive`; nothing is read before the first [`index`](Self::index).
    #[must_use]
    pub const fn new(archive: &'a Archive) -> Self {
        Self {
            archive,
            index: OnceLock::new(),
        }
    }

    /// Returns the Archive.
    #[must_use]
    pub const fn archive(&self) -> &'a Archive {
        self.archive
    }

    /// Returns the audio index, building it on the first call; later calls, from any thread,
    /// return the same index. Callers racing the first call may each build one; the first
    /// stored wins and the others are dropped.
    pub fn index(&self) -> &AudioIndex {
        if let Some(index) = self.index.get() {
            return index;
        }
        let built = AudioIndex::build(self.archive);
        self.index.get_or_init(|| built)
    }
}
