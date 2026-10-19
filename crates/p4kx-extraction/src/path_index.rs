use p4kx_archive::{Archive, Entry};
use rayon::prelude::*;

use crate::entry_path::same_entry_path;

const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

/// The Entries of an Archive looked up by path, ignoring ASCII case and the separator kind.
///
/// Building the index hashes every path once; each lookup is then a binary search.
#[derive(Debug)]
pub struct PathIndex<'a> {
    archive: &'a Archive,
    keys: Vec<(u64, u32)>,
}

impl<'a> PathIndex<'a> {
    /// Builds the index of `archive`'s Entries.
    #[must_use]
    pub fn new(archive: &'a Archive) -> Self {
        let mut keys: Vec<(u64, u32)> = archive
            .entries()
            .par_iter()
            .enumerate()
            .filter_map(|(position, entry)| {
                Some((path_key(entry.path()), u32::try_from(position).ok()?))
            })
            .collect();
        keys.par_sort_unstable();
        Self { archive, keys }
    }

    /// Returns the Archive the index is over.
    #[must_use]
    pub const fn archive(&self) -> &'a Archive {
        self.archive
    }

    /// Returns the Entry at `path`, or `None` when the Archive holds none.
    #[must_use]
    pub fn find(&self, path: &str) -> Option<&'a Entry> {
        let key = path_key(path);
        let start = self.keys.partition_point(|&(candidate, _)| candidate < key);
        let entries = self.archive.entries();
        self.keys
            .get(start..)
            .unwrap_or_default()
            .iter()
            .take_while(|&&(candidate, _)| candidate == key)
            .filter_map(|&(_, position)| entries.get(usize::try_from(position).ok()?))
            .find(|entry| same_entry_path(entry.path(), path))
    }
}

fn path_key(path: &str) -> u64 {
    path.bytes().fold(FNV_OFFSET, |hash, byte| {
        let byte = if byte == b'/' {
            b'\\'
        } else {
            byte.to_ascii_lowercase()
        };
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    })
}
