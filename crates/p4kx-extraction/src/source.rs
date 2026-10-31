// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The files a run reads: the Entries of an Archive or the files of a folder.

use std::borrow::Cow;
use std::fmt::Debug;
use std::io::{self, Read};
use std::path::Path;

use p4kx_archive::{Archive, Checksum, Entry};
use rayon::prelude::*;

use crate::{DataForgeEntry, ExtractionError};

/// Where the data of a [`SourceEntry`] is stored.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Handle<'a> {
    /// An Entry of an Archive.
    Archive(&'a Entry),
    /// A file of a folder, by its path on disk.
    File(&'a Path),
}

/// One file of a run, an Entry of an Archive or a file of a folder, named by its path with `\` as
/// separator.
#[derive(Debug, Clone)]
pub(crate) struct SourceEntry<'a> {
    path: Cow<'a, str>,
    size: u64,
    index: usize,
    handle: Handle<'a>,
}

impl<'a> SourceEntry<'a> {
    /// Returns the file at `path` on disk, `size` bytes long, at position `index` of its source.
    pub(crate) const fn file(path: &'a str, size: u64, index: usize, disk: &'a Path) -> Self {
        Self {
            path: Cow::Borrowed(path),
            size,
            index,
            handle: Handle::File(disk),
        }
    }

    /// Returns the Entry `entry` of an Archive.
    pub(crate) fn of_archive(entry: &'a Entry) -> Self {
        Self {
            path: Cow::Borrowed(entry.path()),
            size: entry.uncompressed_size(),
            index: entry.index(),
            handle: Handle::Archive(entry),
        }
    }

    /// Returns the path, with `\` as separator.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    /// Returns the size of the data.
    pub(crate) const fn uncompressed_size(&self) -> u64 {
        self.size
    }

    /// Returns the position in the source the file was listed at.
    pub(crate) const fn index(&self) -> usize {
        self.index
    }

    /// Returns the checksum its source states, or `None` for a file of a folder.
    pub(crate) const fn checksum(&self) -> Option<Checksum> {
        match self.handle {
            Handle::Archive(entry) => Some(entry.checksum()),
            Handle::File(_) => None,
        }
    }

    pub(crate) const fn handle(&self) -> Handle<'a> {
        self.handle
    }
}

/// The reader of the data of one file.
pub(crate) type SourceReader<'a> = Box<dyn Read + 'a>;

/// Where a run reads its files from.
pub(crate) trait EntrySource: Debug + Sync {
    /// Opens the data of `entry`, checked against its stated checksum when `verify` is set and
    /// the source states one.
    fn open(
        &self,
        entry: &SourceEntry<'_>,
        verify: bool,
    ) -> Result<SourceReader<'_>, ExtractionError>;

    /// Returns the path bytes as stored of the file at `index` when its path is not UTF-8.
    fn raw_path(&self, index: usize) -> Option<&[u8]>;

    /// Returns the path of the file at `index`.
    fn path_of(&self, index: usize) -> Option<&str>;

    /// Returns every file whose path `wanted` accepts, in parallel over the source.
    fn members(&self, wanted: &(dyn Fn(&str) -> bool + Sync)) -> Vec<SourceEntry<'_>>;

    /// Returns the position in `entries` of the DataForge file a Conversion turns into Records.
    fn dataforge_position(&self, entries: &[SourceEntry<'_>]) -> Option<usize>;
}

impl EntrySource for Archive {
    fn open(
        &self,
        entry: &SourceEntry<'_>,
        verify: bool,
    ) -> Result<SourceReader<'_>, ExtractionError> {
        let Handle::Archive(entry) = entry.handle() else {
            return Err(ExtractionError::Read(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the file is not an Entry of this Archive",
            )));
        };
        let reader = if verify {
            self.open_entry_verified(entry)?
        } else {
            self.open_entry(entry)?
        };
        Ok(Box::new(reader))
    }

    fn raw_path(&self, index: usize) -> Option<&[u8]> {
        Self::raw_path(self, index)
    }

    fn path_of(&self, index: usize) -> Option<&str> {
        self.entries().get(index).map(Entry::path)
    }

    fn members(&self, wanted: &(dyn Fn(&str) -> bool + Sync)) -> Vec<SourceEntry<'_>> {
        self.entries()
            .par_iter()
            .filter(|entry| wanted(entry.path()))
            .map(SourceEntry::of_archive)
            .collect()
    }

    fn dataforge_position(&self, entries: &[SourceEntry<'_>]) -> Option<usize> {
        let located = DataForgeEntry::locate(self)?;
        entries
            .iter()
            .position(|entry| entry.path() == located.path())
    }
}
