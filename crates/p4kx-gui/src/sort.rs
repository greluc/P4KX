// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::cmp::Ordering;

use p4kx_extraction::{Compression, Entry};
use rayon::prelude::*;

/// A column of the Entry table that Entries can be sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortColumn {
    /// The file name of the Entry, ignoring case.
    Name,
    /// The compression method.
    Compression,
    /// Whether the Entry is encrypted.
    Encryption,
    /// The size after decompression.
    Size,
}

/// The direction of a sort.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortDirection {
    /// Smallest first.
    Ascending,
    /// Largest first.
    Descending,
}

impl SortDirection {
    /// Returns the opposite direction.
    #[must_use]
    pub const fn reversed(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

/// How the Entry table is sorted; ties are in path order, reversed when descending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sort {
    /// The column sorted by.
    pub column: SortColumn,
    /// The direction.
    pub direction: SortDirection,
}

impl Default for Sort {
    fn default() -> Self {
        Self {
            column: SortColumn::Name,
            direction: SortDirection::Ascending,
        }
    }
}

/// Every Entry id once per column, in ascending order of that column.
#[derive(Debug, Default)]
pub(crate) struct SortOrders {
    name: Vec<u32>,
    compression: Vec<u32>,
    encryption: Vec<u32>,
    size: Vec<u32>,
}

impl SortOrders {
    /// Sorts the ids of `entries`, which are in path order, by every column; `total_size`
    /// gives the size of a whole split texture for its header.
    pub(crate) fn build(
        entries: &[&Entry],
        total_size: impl Fn(u32) -> Option<u64> + Sync,
    ) -> Self {
        let ids: Vec<u32> = (0..entries.len())
            .filter_map(|id| u32::try_from(id).ok())
            .collect();
        let entry = |id: u32| entries.get(id as usize).copied();
        let ((name, compression), (encryption, size)) = rayon::join(
            || {
                rayon::join(
                    || sorted(&ids, |a, b| compare_names(entry(a), entry(b))),
                    || sorted_by_key(&ids, |id| entry(id).map(|e| rank(e.compression()))),
                )
            },
            || {
                rayon::join(
                    || sorted_by_key(&ids, |id| entry(id).map(Entry::is_encrypted)),
                    || {
                        sorted_by_key(&ids, |id| {
                            total_size(id).or_else(|| entry(id).map(Entry::uncompressed_size))
                        })
                    },
                )
            },
        );
        Self {
            name,
            compression,
            encryption,
            size,
        }
    }

    /// Returns the ids in ascending order of `column`.
    pub(crate) fn ascending(&self, column: SortColumn) -> &[u32] {
        match column {
            SortColumn::Name => &self.name,
            SortColumn::Compression => &self.compression,
            SortColumn::Encryption => &self.encryption,
            SortColumn::Size => &self.size,
        }
    }
}

/// Compares two names byte by byte with ASCII letters folded to lower case.
pub(crate) fn compare_ignoring_case(a: &str, b: &str) -> Ordering {
    a.bytes()
        .map(|byte| byte.to_ascii_lowercase())
        .cmp(b.bytes().map(|byte| byte.to_ascii_lowercase()))
}

/// Returns the file name of an Entry path, the part after the last backslash or slash.
pub(crate) fn file_name(path: &str) -> &str {
    path.bytes()
        .rposition(|byte| byte == b'\\' || byte == b'/')
        .and_then(|separator| path.get(separator + 1..))
        .unwrap_or(path)
}

fn compare_names(a: Option<&Entry>, b: Option<&Entry>) -> Ordering {
    compare_ignoring_case(entry_name(a), entry_name(b))
}

fn entry_name(entry: Option<&Entry>) -> &str {
    entry.map_or("", |entry| file_name(entry.path()))
}

const fn rank(compression: Compression) -> u32 {
    match compression {
        Compression::Store => 0,
        Compression::Deflate => 1,
        Compression::Zstd => 2,
        Compression::Other(method) => 3 + method as u32,
    }
}

fn sorted(ids: &[u32], compare: impl Fn(u32, u32) -> Ordering + Sync) -> Vec<u32> {
    let mut ids = ids.to_vec();
    ids.par_sort_by(|&a, &b| compare(a, b));
    ids
}

fn sorted_by_key<K: Ord + Send>(ids: &[u32], key: impl Fn(u32) -> K + Sync) -> Vec<u32> {
    let mut ids = ids.to_vec();
    ids.par_sort_by_key(|&id| key(id));
    ids
}
