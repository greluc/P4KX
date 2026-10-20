use std::io::Read;

use p4kx_extraction::Entry;

use super::PREVIEW_READ_LIMIT;
use crate::model::describe;

/// Reads at most [`PREVIEW_READ_LIMIT`] bytes from the start of `entry`.
pub(super) fn read_start(reader: &mut impl Read, entry: &Entry) -> Result<Vec<u8>, String> {
    let limit = u64::try_from(PREVIEW_READ_LIMIT).unwrap_or(u64::MAX);
    let capacity = usize::try_from(entry.uncompressed_size().min(limit)).unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    reader
        .by_ref()
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|error| describe(&error))?;
    Ok(bytes)
}
