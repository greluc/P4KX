use std::io::{self, Read};

use p4kx_foundation::ByteReader;

use crate::format_error::FormatError;
use crate::source::Source;
use crate::{Compression, Entry};

const LOCAL_HEADER_SIGNATURE: u32 = 0x0403_4B50;
const LOCAL_HEADER_CIG_SIGNATURE: u32 = 0x1403_4B50;
const LOCAL_HEADER_LEN: usize = 30;
const LOCAL_HEADER_FIXED_FIELDS_LEN: usize = 22;

/// Streams the data of one Entry from its Archive.
#[derive(Debug)]
pub struct EntryReader<'a> {
    source: &'a Source,
    offset: u64,
    remaining: u64,
}

impl Read for EntryReader<'_> {
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

/// Opens the data of `entry` after checking its local header.
pub(crate) fn open<'a>(source: &'a Source, entry: &Entry) -> Result<EntryReader<'a>, FormatError> {
    if entry.encrypted {
        return Err(FormatError::Encrypted {
            entry: entry.path.clone(),
        });
    }
    if entry.compression != Compression::Store {
        return Err(FormatError::UnsupportedCompression {
            entry: entry.path.clone(),
            compression: entry.compression,
        });
    }
    let offset = entry.local_header_offset;
    let data_offset = read_local_header(source, offset)?;
    if !fits(source, data_offset, entry.compressed_size) {
        return Err(FormatError::corrupt(
            offset,
            "Entry data lies outside the file",
        ));
    }
    Ok(EntryReader {
        source,
        offset: data_offset,
        remaining: entry.compressed_size,
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

fn fits(source: &Source, offset: u64, len: u64) -> bool {
    offset
        .checked_add(len)
        .is_some_and(|end| end <= source.len())
}
