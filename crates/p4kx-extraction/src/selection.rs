// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::atomic::{AtomicBool, Ordering};

use p4kx_archive::{Archive, Entry};
use rayon::prelude::*;
use thiserror::Error;

use crate::entry_path::same_entry_path;
use crate::path_index::path_key;

/// Requested Entry paths that name no Entry of the Archive.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("no Entry named {}", .0.join(", "))]
pub struct UnknownEntries(pub Vec<String>);

/// Returns the Entries named by `requested`, in Archive order.
///
/// Paths match with either `/` or `\` as separator and ignore ASCII case. The Entries are scanned
/// in parallel without allocating per Entry.
///
/// # Errors
///
/// Returns [`UnknownEntries`] with every requested path that names no Entry.
pub fn select_entries<'a>(
    archive: &'a Archive,
    requested: &[impl AsRef<str>],
) -> Result<Vec<&'a Entry>, UnknownEntries> {
    let requested: Vec<&str> = requested.iter().map(AsRef::as_ref).collect();
    let mut wanted: Vec<(u64, usize)> = requested
        .iter()
        .enumerate()
        .map(|(position, path)| (path_key(path), position))
        .collect();
    wanted.sort_unstable();
    let found: Vec<AtomicBool> = requested.iter().map(|_| AtomicBool::new(false)).collect();
    let selected: Vec<&Entry> = archive
        .entries()
        .par_iter()
        .filter(|entry| {
            let key = path_key(entry.path());
            let start = wanted.partition_point(|&(candidate, _)| candidate < key);
            let mut hit = false;
            for &(candidate, position) in wanted.get(start..).unwrap_or_default() {
                if candidate != key {
                    break;
                }
                let matches = requested
                    .get(position)
                    .is_some_and(|path| same_entry_path(entry.path(), path));
                if matches && let Some(flag) = found.get(position) {
                    flag.store(true, Ordering::Relaxed);
                    hit = true;
                }
            }
            hit
        })
        .collect();
    let unknown: Vec<String> = requested
        .iter()
        .zip(&found)
        .filter(|(_, flag)| !flag.load(Ordering::Relaxed))
        .map(|(path, _)| (*path).to_owned())
        .collect();
    if unknown.is_empty() {
        Ok(selected)
    } else {
        Err(UnknownEntries(unknown))
    }
}
