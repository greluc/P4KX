use std::io::{self, Read};

use p4kx_foundation::ArchiveKey;

use crate::format_error::FormatError;
use crate::source::Source;
use crate::{DataError, Entry, entry_reader};

/// The largest encrypted Entry, in stored bytes, a key check reads.
const MAX_CHECK_LEN: u64 = 16 * 1024 * 1024;
const CHUNK_LEN: usize = 64 * 1024;

/// What checking a key against an Archive found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyCheck {
    /// The key decrypted an encrypted Entry to data matching its checksum.
    Verified,
    /// The Archive has no encrypted Entry suitable for the check, so the key is neither
    /// confirmed nor refuted.
    Unchecked,
}

pub(crate) enum Verdict {
    Verified,
    Unchecked,
    Wrong { entry: String },
}

/// Decrypts the smallest suitable encrypted Entry with `key` and compares the CRC-32C of the
/// result with the directory's.
pub(crate) fn verify(
    source: &Source,
    entries: &[Entry],
    key: &ArchiveKey,
) -> Result<Verdict, FormatError> {
    let Some(entry) = entries
        .iter()
        .filter(|entry| {
            entry.encrypted
                && entry.uncompressed_size > 0
                && entry.crc32c().is_some()
                && entry.compressed_size <= MAX_CHECK_LEN
        })
        .min_by_key(|entry| entry.compressed_size)
    else {
        return Ok(Verdict::Unchecked);
    };
    let wrong = || Verdict::Wrong {
        entry: entry.path.clone(),
    };
    let (mut reader, _) = entry_reader::open_at(source, entry, 0, Some(key))?;
    let mut chunk = vec![0; CHUNK_LEN];
    let mut crc = 0_u32;
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => crc = crc32c::crc32c_append(crc, chunk.get(..count).unwrap_or_default()),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) if is_data_error(&error) => return Ok(wrong()),
            Err(error) => return Err(error.into()),
        }
    }
    if Some(crc) == entry.crc32c() {
        Ok(Verdict::Verified)
    } else {
        Ok(wrong())
    }
}

fn is_data_error(error: &io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(|inner| inner.downcast_ref::<DataError>().is_some())
}
