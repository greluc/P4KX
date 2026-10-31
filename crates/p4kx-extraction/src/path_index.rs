// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;

use p4kx_archive::{Archive, Entry};
use rayon::prelude::*;

use crate::entry_path::same_entry_path;
use crate::texture_grey::GreyVerdicts;

const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

/// The Entries of an Archive looked up by path, ignoring ASCII case and the separator kind.
///
/// Building the index hashes every path once; each lookup is then a binary search. The index also
/// keeps the colour verdicts of the textures it has judged.
#[derive(Debug)]
pub struct PathIndex<'a> {
    archive: &'a Archive,
    keys: Cow<'a, [(u64, u32)]>,
    pub(crate) grey: GreyVerdicts,
}

/// The sorted path hashes of an Archive's Entries, kept apart from the Archive so one build serves
/// every lookup: [`PathKeys::index`] lends them as a [`PathIndex`].
#[derive(Debug, Clone)]
pub struct PathKeys {
    keys: Vec<(u64, u32)>,
}

impl PathKeys {
    /// Hashes every path of `archive`'s Entries once.
    #[must_use]
    pub fn new(archive: &Archive) -> Self {
        let mut keys: Vec<(u64, u32)> = archive
            .entries()
            .par_iter()
            .enumerate()
            .filter_map(|(position, entry)| {
                Some((path_key(entry.path()), u32::try_from(position).ok()?))
            })
            .collect();
        keys.par_sort_unstable();
        Self { keys }
    }

    /// Returns the index over `archive`, which must be the Archive these keys were built from; for
    /// another one lookups find nothing or an Entry whose path really matches.
    #[must_use]
    pub fn index<'a>(&'a self, archive: &'a Archive) -> PathIndex<'a> {
        PathIndex {
            archive,
            keys: Cow::Borrowed(&self.keys),
            grey: GreyVerdicts::default(),
        }
    }
}

impl<'a> PathIndex<'a> {
    /// Builds the index of `archive`'s Entries.
    #[must_use]
    pub fn new(archive: &'a Archive) -> Self {
        Self {
            archive,
            keys: Cow::Owned(PathKeys::new(archive).keys),
            grey: GreyVerdicts::default(),
        }
    }

    /// Returns the Archive the index is over.
    #[must_use]
    pub const fn archive(&self) -> &'a Archive {
        self.archive
    }

    /// Returns the Entry at `path`, or `None` when the Archive holds none.
    #[must_use]
    pub fn find(&self, path: &str) -> Option<&'a Entry> {
        self.archive.entries().get(self.position(path)?)
    }

    /// Returns the position in [`Archive::entries`] of the first Entry at `path`, or `None`
    /// when the Archive holds none.
    #[must_use]
    pub fn position(&self, path: &str) -> Option<usize> {
        let key = path_key(path);
        let start = self.keys.partition_point(|&(candidate, _)| candidate < key);
        let entries = self.archive.entries();
        self.keys
            .get(start..)
            .unwrap_or_default()
            .iter()
            .take_while(|&&(candidate, _)| candidate == key)
            .filter_map(|&(_, position)| usize::try_from(position).ok())
            .find(|&position| {
                entries
                    .get(position)
                    .is_some_and(|entry| same_entry_path(entry.path(), path))
            })
    }
}

pub(crate) fn path_key(path: &str) -> u64 {
    path.bytes().fold(FNV_OFFSET, |hash, byte| {
        let byte = if byte == b'/' {
            b'\\'
        } else {
            byte.to_ascii_lowercase()
        };
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    })
}
