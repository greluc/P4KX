use p4kx_foundation::{ByteReader, ParseError};

use crate::source::Source;

const END: u32 = 0x0605_4B50;
const ZIP64_LOCATOR: u32 = 0x0706_4B50;
const ZIP64_END: u32 = 0x0606_4B50;
const END_LEN: usize = 22;
const ZIP64_LOCATOR_LEN: usize = 20;
const ZIP64_END_LEN: usize = 56;
const MAX_COMMENT_LEN: usize = u16::MAX as usize;

/// Where the central directory lies and how many Entries it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DirectoryLocation {
    pub(crate) entry_count: u64,
    pub(crate) offset: u64,
    pub(crate) size: u64,
}

/// Why the end records could not be read.
#[derive(Debug)]
pub(crate) enum EndRecordsError {
    Io(std::io::Error),
    NotFound,
    Corrupt { offset: u64, reason: &'static str },
    Truncated(ParseError),
}

impl From<std::io::Error> for EndRecordsError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ParseError> for EndRecordsError {
    fn from(error: ParseError) -> Self {
        Self::Truncated(error)
    }
}

/// Reads the end-of-central-directory record and the ZIP64 records it points to.
pub(crate) fn locate_directory(source: &Source) -> Result<DirectoryLocation, EndRecordsError> {
    let tail_len = source.len().min((END_LEN + MAX_COMMENT_LEN) as u64);
    let tail_start = source.len() - tail_len;
    let tail = source.read_at(tail_start, usize_from(tail_len))?;
    let end_index = find_end_record(&tail).ok_or(EndRecordsError::NotFound)?;
    let end_offset = tail_start + end_index as u64;
    let locator_offset =
        end_offset
            .checked_sub(ZIP64_LOCATOR_LEN as u64)
            .ok_or(EndRecordsError::Corrupt {
                offset: end_offset,
                reason: "no ZIP64 locator before the end record",
            })?;
    let locator = source.read_at(locator_offset, ZIP64_LOCATOR_LEN)?;
    let mut reader = ByteReader::with_base(&locator, locator_offset);
    if reader.read_u32()? != ZIP64_LOCATOR {
        return Err(EndRecordsError::Corrupt {
            offset: locator_offset,
            reason: "ZIP64 locator signature missing",
        });
    }
    reader.skip(4)?;
    let zip64_end_offset = reader.read_u64()?;
    read_zip64_end(source, zip64_end_offset)
}

fn read_zip64_end(source: &Source, offset: u64) -> Result<DirectoryLocation, EndRecordsError> {
    if offset
        .checked_add(ZIP64_END_LEN as u64)
        .is_none_or(|end| end > source.len())
    {
        return Err(EndRecordsError::Corrupt {
            offset,
            reason: "ZIP64 end record lies outside the file",
        });
    }
    let bytes = source.read_at(offset, ZIP64_END_LEN)?;
    let mut reader = ByteReader::with_base(&bytes, offset);
    if reader.read_u32()? != ZIP64_END {
        return Err(EndRecordsError::Corrupt {
            offset,
            reason: "ZIP64 end record signature missing",
        });
    }
    reader.skip(28)?;
    let entry_count = reader.read_u64()?;
    let size = reader.read_u64()?;
    let directory_offset = reader.read_u64()?;
    if directory_offset
        .checked_add(size)
        .is_none_or(|end| end > offset)
    {
        return Err(EndRecordsError::Corrupt {
            offset,
            reason: "central directory lies outside the file",
        });
    }
    Ok(DirectoryLocation {
        entry_count,
        offset: directory_offset,
        size,
    })
}

fn find_end_record(tail: &[u8]) -> Option<usize> {
    let signature = END.to_le_bytes();
    let last_start = tail.len().checked_sub(END_LEN)?;
    (0..=last_start)
        .rev()
        .find(|&index| tail.get(index..index + 4) == Some(signature.as_slice()))
}

fn usize_from(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}
