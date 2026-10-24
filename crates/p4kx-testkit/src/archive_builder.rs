// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io;
use std::path::Path;

use crate::{TestEntry, layout_v1, layout_v2};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Layout {
    #[default]
    V1,
    V2,
}

/// Builds a synthetic Archive in the byte layout of real Star Citizen Archives.
///
/// The v1 layout writes every size and offset through the ZIP64 extra block, followed by the CIG
/// blocks 0x5000, 0x5002 and 0x5003, whose length fields include their 4-byte header. The v2
/// layout writes the data without local headers, 204-byte directory records, a name table and the
/// 175-byte trailer ending in `Jiji`.
#[derive(Debug, Clone, Default)]
pub struct ArchiveBuilder {
    layout: Layout,
    entries: Vec<TestEntry>,
}

impl ArchiveBuilder {
    /// Creates a builder for a v1 Archive without Entries.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a builder for a v2 Archive without Entries.
    #[must_use]
    pub const fn v2() -> Self {
        Self {
            layout: Layout::V2,
            entries: Vec::new(),
        }
    }

    /// Adds an Entry.
    #[must_use]
    pub fn entry(mut self, entry: TestEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Returns the bytes of the Archive.
    #[must_use]
    pub fn build(&self) -> Vec<u8> {
        match self.layout {
            Layout::V1 => layout_v1::build(&self.entries),
            Layout::V2 => layout_v2::build(&self.entries),
        }
    }

    /// Writes the Archive to `path`.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of writing the file.
    pub fn write_to(&self, path: impl AsRef<Path>) -> io::Result<()> {
        std::fs::write(path, self.build())
    }
}
