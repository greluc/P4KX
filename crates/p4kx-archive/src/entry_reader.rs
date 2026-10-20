use std::fmt;
use std::io::{self, BufReader, Read};

use flate2::read::DeflateDecoder;
use p4kx_foundation::{ARCHIVE_IV, ByteReader};

use crate::decode::{CorruptAs, ExactSize, Overlong, ZstdOrPlain};
use crate::decrypt::{AES_BLOCK_LEN, Decrypting};
use crate::entry::DataStart;
use crate::format_error::FormatError;
use crate::source::Source;
use crate::{Compression, Entry};

const LOCAL_HEADER_SIGNATURE: u32 = 0x0403_4B50;
const LOCAL_HEADER_CIG_SIGNATURE: u32 = 0x1403_4B50;
const LOCAL_HEADER_LEN: usize = 30;
const LOCAL_HEADER_FIXED_FIELDS_LEN: usize = 22;
const DECODER_BUFFER_LEN: usize = 256 * 1024;

type DataReader<'a> = Box<dyn Read + Send + 'a>;

/// Streams the decrypted and decompressed data of one Entry from its Archive.
///
/// Decoding errors are reported as [`io::Error`]s of kind [`io::ErrorKind::InvalidData`] that
/// carry a [`DataError`](crate::DataError).
pub struct EntryReader<'a> {
    inner: ExactSize<DataReader<'a>>,
    encrypted: bool,
}

impl EntryReader<'_> {
    /// Makes data that goes on after the Entry's uncompressed size an error, a
    /// [`DataError::TooLong`](crate::DataError::TooLong), instead of cutting it; the zero padding
    /// of an encrypted Entry below one AES block is still allowed.
    #[must_use]
    pub fn reject_overlong(mut self) -> Self {
        self.inner.set_overlong(Overlong::Reject {
            padding: self.encrypted,
        });
        self
    }
}

impl fmt::Debug for EntryReader<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EntryReader").finish_non_exhaustive()
    }
}

impl Read for EntryReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buffer)
    }
}

/// Reads the stored bytes of an Entry by positional reads.
struct RawReader<'a> {
    source: &'a Source,
    offset: u64,
    remaining: u64,
}

impl Read for RawReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = usize::try_from(self.remaining)
            .unwrap_or(usize::MAX)
            .min(buffer.len());
        let Some(chunk) = buffer.get_mut(..count).filter(|chunk| !chunk.is_empty()) else {
            return Ok(0);
        };
        self.source.read_into(self.offset, chunk)?;
        self.offset += count as u64;
        self.remaining -= count as u64;
        Ok(count)
    }
}

/// Returns where the stored data of `entry` begins after checking its local header, if its layout
/// has one, and that the data lies inside `source`.
pub(crate) fn data_offset(source: &Source, entry: &Entry) -> Result<u64, FormatError> {
    let (offset, data_offset) = match entry.data() {
        DataStart::LocalHeader(offset) => (offset, read_local_header(source, offset)?),
        DataStart::Direct(offset) => (offset, offset),
    };
    if !fits(source, data_offset, entry.compressed_size) {
        return Err(FormatError::corrupt(
            offset,
            "Entry data lies outside the file",
        ));
    }
    Ok(data_offset)
}

/// Opens the data of `entry` after checking its local header, if its layout has one.
pub(crate) fn open<'a>(source: &'a Source, entry: &Entry) -> Result<EntryReader<'a>, FormatError> {
    open_at(source, entry, 0).map(|(reader, _)| reader)
}

/// Opens the data of `entry` from decoded byte `offset`, at most its size; returns the reader
/// and how many decoded bytes it still has to skip to reach `offset`.
///
/// Stored data starts at `offset` directly: unencrypted at that byte, encrypted at the AES block
/// holding it, chained from the previous ciphertext block. Compressed data starts at the
/// beginning and the whole `offset` is left to skip.
pub(crate) fn open_at<'a>(
    source: &'a Source,
    entry: &Entry,
    offset: u64,
) -> Result<(EntryReader<'a>, u64), FormatError> {
    let data_offset = data_offset(source, entry)?;
    if entry.encrypted && !entry.compressed_size.is_multiple_of(AES_BLOCK_LEN as u64) {
        return Err(FormatError::corrupt(
            entry.data_offset,
            "encrypted Entry data is not a whole number of AES blocks",
        ));
    }
    let start = match entry.compression {
        Compression::Store if entry.encrypted => {
            offset.min(entry.compressed_size) / AES_BLOCK_LEN as u64 * AES_BLOCK_LEN as u64
        }
        Compression::Store => offset.min(entry.compressed_size),
        _ => 0,
    };
    let iv = if entry.encrypted && start > 0 {
        let mut iv = [0; AES_BLOCK_LEN];
        source.read_into(data_offset + start - AES_BLOCK_LEN as u64, &mut iv)?;
        iv
    } else {
        ARCHIVE_IV
    };
    let stored_len = entry.compressed_size - start;
    let raw = RawReader {
        source,
        offset: data_offset + start,
        remaining: stored_len,
    };
    let stored: DataReader<'a> = if entry.encrypted {
        Box::new(Decrypting::new(raw, buffer_len(stored_len), iv))
    } else {
        Box::new(raw)
    };
    let decoded = decoder(stored, entry)?;
    let reader = EntryReader {
        inner: ExactSize::new(decoded, entry.uncompressed_size.saturating_sub(start)),
        encrypted: entry.encrypted,
    };
    Ok((reader, offset.saturating_sub(start)))
}

/// Wraps `stored` in the decompressor of `entry`.
fn decoder<'a>(stored: DataReader<'a>, entry: &Entry) -> Result<DataReader<'a>, FormatError> {
    Ok(match entry.compression {
        Compression::Store => stored,
        Compression::Deflate => Box::new(CorruptAs::new(
            DeflateDecoder::new_with_buf(stored, vec![0; buffer_len(entry.compressed_size)]),
            entry.compression,
        )),
        Compression::Zstd => Box::new(CorruptAs::new(
            ZstdOrPlain::new(
                BufReader::with_capacity(buffer_len(entry.compressed_size), stored),
                entry.uncompressed_size,
            ),
            entry.compression,
        )),
        Compression::Other(_) => {
            return Err(FormatError::UnsupportedCompression {
                entry: entry.path.clone(),
                compression: entry.compression,
            });
        }
    })
}

fn read_local_header(source: &Source, offset: u64) -> Result<u64, FormatError> {
    if !fits(source, offset, LOCAL_HEADER_LEN as u64) {
        return Err(FormatError::corrupt(
            offset,
            "local header lies outside the file",
        ));
    }
    let header = source.read_at(offset, LOCAL_HEADER_LEN)?;
    let mut reader = ByteReader::with_base(&header, offset);
    let signature = reader.read_u32()?;
    if signature != LOCAL_HEADER_SIGNATURE && signature != LOCAL_HEADER_CIG_SIGNATURE {
        return Err(FormatError::corrupt(
            offset,
            "local header signature missing",
        ));
    }
    reader.skip(LOCAL_HEADER_FIXED_FIELDS_LEN)?;
    let name_len = u64::from(reader.read_u16()?);
    let extra_len = u64::from(reader.read_u16()?);
    Ok(offset + LOCAL_HEADER_LEN as u64 + name_len + extra_len)
}

fn buffer_len(compressed_size: u64) -> usize {
    usize::try_from(compressed_size)
        .unwrap_or(usize::MAX)
        .clamp(1, DECODER_BUFFER_LEN)
}

fn fits(source: &Source, offset: u64, len: u64) -> bool {
    offset
        .checked_add(len)
        .is_some_and(|end| end <= source.len())
}
