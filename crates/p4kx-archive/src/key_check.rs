// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Read};

use p4kx_foundation::ArchiveKey;

use crate::source::Source;
use crate::{DataError, Entry, entry_reader};

/// The largest encrypted Entry, in stored bytes, a key check reads.
const MAX_CHECK_LEN: u64 = 16 * 1024 * 1024;
/// How many of the smallest suitable Entries a key must fail on before it is called wrong.
const MAX_CANDIDATES: usize = 3;
const CHUNK_LEN: usize = 64 * 1024;

/// What checking a key against an Archive found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyCheck {
    /// The key decrypted an encrypted Entry to data matching its checksum.
    Verified,
    /// The Archive has no encrypted Entry suitable for the check, or none could be read, so the
    /// key is neither confirmed nor refuted.
    Unchecked,
}

pub(crate) enum Verdict {
    Verified,
    Unchecked,
    Wrong { entry: String },
}

enum Attempt {
    Matches,
    DataMismatch,
    Unreadable,
}

/// Decrypts the smallest suitable encrypted Entries with `key` and compares the CRC-32C of the
/// result with the directory's. The key is wrong only when it fails on every candidate (at most
/// three) and each could be read; an Entry that cannot be read leaves the key unchecked.
pub(crate) fn verify(source: &Source, entries: &[Entry], key: &ArchiveKey) -> Verdict {
    let mut candidates: Vec<&Entry> = entries
        .iter()
        .filter(|entry| {
            entry.encrypted
                && entry.uncompressed_size > 0
                && entry.crc32c().is_some()
                && entry.compressed_size <= MAX_CHECK_LEN
        })
        .collect();
    candidates.sort_by_key(|entry| entry.compressed_size);
    candidates.truncate(MAX_CANDIDATES);
    let mut chunk = vec![0; CHUNK_LEN];
    let mut unreadable = false;
    for entry in &candidates {
        match attempt(source, entry, key, &mut chunk) {
            Attempt::Matches => return Verdict::Verified,
            Attempt::DataMismatch => {}
            Attempt::Unreadable => unreadable = true,
        }
    }
    match candidates.first() {
        Some(first) if !unreadable => Verdict::Wrong {
            entry: first.path.clone(),
        },
        _ => Verdict::Unchecked,
    }
}

fn attempt(source: &Source, entry: &Entry, key: &ArchiveKey, chunk: &mut [u8]) -> Attempt {
    let Ok((reader, _)) = entry_reader::open_at(source, entry, 0, Some(key)) else {
        return Attempt::Unreadable;
    };
    let mut reader = reader.verified(entry);
    loop {
        match reader.read(chunk) {
            Ok(0) => return Attempt::Matches,
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) if is_data_error(&error) => return Attempt::DataMismatch,
            Err(_) => return Attempt::Unreadable,
        }
    }
}

fn is_data_error(error: &io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(|inner| inner.downcast_ref::<DataError>().is_some())
}
