use std::io::Read;

use crate::entry::DataStart;
use crate::entry_reader;
use crate::format_error::FormatError;
use crate::source::Source;
use crate::{Checksum, Compression, Entry};

/// How many containers deep [`Archive::open_container`](crate::Archive::open_container) opens.
pub const MAX_CONTAINER_DEPTH: usize = 8;
/// How many Entries a container may hold.
pub const MAX_CONTAINER_ENTRIES: usize = 1 << 20;
/// How many bytes a compressed or encrypted container may decode to; it is held in memory.
pub const MAX_CONTAINER_LEN: u64 = 512 * 1024 * 1024;
/// How many times its stored size an Entry of a container may declare as its uncompressed size.
pub const MAX_COMPRESSION_RATIO: u64 = 2048;

const LOCAL_HEADER_LEN: u64 = 30;

/// Returns the bytes of the container held in `entry`: a window of `parent` when the Entry is
/// stored unencrypted, otherwise its data decoded into memory.
pub(crate) fn source_of(parent: &Source, entry: &Entry) -> Result<Source, FormatError> {
    if entry.compression == Compression::Store && !entry.encrypted {
        let offset = entry_reader::data_offset(parent, entry)?;
        if entry.compressed_size != entry.uncompressed_size {
            return Err(stored_sizes_differ(offset, entry));
        }
        return parent
            .slice(offset, entry.compressed_size)
            .ok_or_else(|| FormatError::corrupt(offset, "Entry data lies outside the file"));
    }
    if entry.uncompressed_size > MAX_CONTAINER_LEN {
        return Err(FormatError::LimitExceeded(format!(
            "it declares {} bytes, larger than the {MAX_CONTAINER_LEN} bytes a compressed \
             container may decode to",
            entry.uncompressed_size
        )));
    }
    let capacity = entry
        .uncompressed_size
        .min(entry.compressed_size.saturating_mul(MAX_COMPRESSION_RATIO));
    let mut bytes = Vec::with_capacity(usize::try_from(capacity).unwrap_or(0));
    entry_reader::open(parent, entry)?.read_to_end(&mut bytes)?;
    Ok(Source::memory(bytes))
}

/// Checks the Entries of a container of `len` bytes against the limits, leaves out folder
/// Entries, writes their paths with backslashes and marks the checksums as the plain ZIP CRC-32
/// that containers store.
pub(crate) fn admit(entries: Vec<Entry>, len: u64) -> Result<Vec<Entry>, FormatError> {
    if entries.len() > MAX_CONTAINER_ENTRIES {
        return Err(FormatError::LimitExceeded(format!(
            "it holds {} Entries, more than {MAX_CONTAINER_ENTRIES}",
            entries.len()
        )));
    }
    let mut admitted = Vec::with_capacity(entries.len());
    for mut entry in entries {
        if entry.path.ends_with(['\\', '/']) && entry.uncompressed_size == 0 {
            continue;
        }
        check_sizes(&entry)?;
        if entry.path.contains('/') {
            entry.path = entry.path.replace('/', "\\");
        }
        entry.checksum = Checksum::Crc32(entry.checksum.value());
        admitted.push(entry);
    }
    check_disjoint(&admitted, len)?;
    Ok(admitted)
}

fn check_sizes(entry: &Entry) -> Result<(), FormatError> {
    if entry.compression == Compression::Store
        && !entry.encrypted
        && entry.compressed_size != entry.uncompressed_size
    {
        return Err(stored_sizes_differ(entry.data.offset(), entry));
    }
    if entry.uncompressed_size > entry.compressed_size.saturating_mul(MAX_COMPRESSION_RATIO) {
        return Err(FormatError::LimitExceeded(format!(
            "Entry {} declares {} bytes from {} stored bytes, beyond the compression ratio limit \
             of {MAX_COMPRESSION_RATIO}",
            entry.path, entry.uncompressed_size, entry.compressed_size
        )));
    }
    Ok(())
}

fn check_disjoint(entries: &[Entry], len: u64) -> Result<(), FormatError> {
    let mut ranges: Vec<(u64, u64)> = entries
        .iter()
        .map(|entry| (entry.data.offset(), min_stored_len(entry)))
        .collect();
    ranges.sort_unstable();
    let mut free_from = 0_u64;
    for (start, stored_len) in ranges {
        if start < free_from {
            return Err(FormatError::corrupt(
                start,
                "Entry data overlaps the data of another Entry",
            ));
        }
        free_from = start
            .checked_add(stored_len)
            .filter(|&end| end <= len)
            .ok_or_else(|| FormatError::corrupt(start, "Entry data lies outside the file"))?;
    }
    Ok(())
}

const fn min_stored_len(entry: &Entry) -> u64 {
    match entry.data {
        DataStart::LocalHeader(_) => entry.compressed_size.saturating_add(LOCAL_HEADER_LEN),
        DataStart::Direct(_) => entry.compressed_size,
    }
}

fn stored_sizes_differ(offset: u64, entry: &Entry) -> FormatError {
    FormatError::corrupt(
        offset,
        format!(
            "stored Entry {} declares {} bytes but stores {}",
            entry.path, entry.uncompressed_size, entry.compressed_size
        ),
    )
}
