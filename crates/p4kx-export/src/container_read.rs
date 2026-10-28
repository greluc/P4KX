// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Reading the files inside a container (`.socpak`).

use std::io::Read;
use std::path::Path;

use p4kx_extraction::{Archive, Entry};

/// The largest `.soc` file that is read, 64 MiB.
pub(crate) const MAX_SOC_LEN: u64 = 64 << 20;

/// The extension of the entity file of a container.
const SOC_EXTENSION: &str = "soc";

/// Returns the `.soc` files of `container`, in stored order.
pub(crate) fn soc_entries(container: &Archive) -> impl Iterator<Item = &Entry> {
    container.entries().iter().filter(|inner| {
        Path::new(inner.path())
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(SOC_EXTENSION))
    })
}

/// Reads `entry` of `container` unless it is larger than `limit` bytes or cannot be read.
pub(crate) fn read_limited(container: &Archive, entry: &Entry, limit: u64) -> Option<Vec<u8>> {
    if entry.uncompressed_size() > limit {
        return None;
    }
    let mut bytes = Vec::new();
    container
        .open_entry(entry)
        .ok()?
        .take(limit)
        .read_to_end(&mut bytes)
        .ok()?;
    Some(bytes)
}
