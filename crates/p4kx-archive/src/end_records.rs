use p4kx_foundation::ByteReader;

use crate::format_error::FormatError;
use crate::source::Source;

const END_SIGNATURE: u32 = 0x0605_4B50;
const ZIP64_LOCATOR_SIGNATURE: u32 = 0x0706_4B50;
const ZIP64_END_SIGNATURE: u32 = 0x0606_4B50;
const END_LEN: usize = 22;
const MAX_COMMENT_LEN: usize = u16::MAX as usize;
const ZIP64_LOCATOR_LEN: usize = 20;
const ZIP64_LOCATOR_DISK_LEN: usize = 4;
const ZIP64_END_LEN: usize = 56;
const ZIP64_END_SIZE_VERSIONS_DISKS_LEN: usize = 28;

/// Where the central directory lies and how many Entries it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DirectoryLocation {
    pub(crate) entry_count: u64,
    pub(crate) offset: u64,
    pub(crate) size: u64,
}

/// Reads the end-of-central-directory record and the ZIP64 records it points to.
pub(crate) fn locate_directory(source: &Source) -> Result<DirectoryLocation, FormatError> {
    let tail_len = source.len().min((END_LEN + MAX_COMMENT_LEN) as u64);
    let tail_start = source.len() - tail_len;
    let tail = source.read_at(tail_start, usize::try_from(tail_len).unwrap_or(usize::MAX))?;
    let end_index = find_end_record(&tail).ok_or(FormatError::NotAnArchive)?;
    let end_offset = tail_start + end_index as u64;
    let locator_offset = end_offset
        .checked_sub(ZIP64_LOCATOR_LEN as u64)
        .ok_or_else(|| {
            FormatError::corrupt(end_offset, "no ZIP64 locator before the end record")
        })?;
    let locator = source.read_at(locator_offset, ZIP64_LOCATOR_LEN)?;
    let mut reader = ByteReader::with_base(&locator, locator_offset);
    if reader.read_u32()? != ZIP64_LOCATOR_SIGNATURE {
        return Err(FormatError::corrupt(
            locator_offset,
            "ZIP64 locator signature missing",
        ));
    }
    reader.skip(ZIP64_LOCATOR_DISK_LEN)?;
    let zip64_end_offset = reader.read_u64()?;
    read_zip64_end(source, zip64_end_offset)
}

fn read_zip64_end(source: &Source, offset: u64) -> Result<DirectoryLocation, FormatError> {
    if offset
        .checked_add(ZIP64_END_LEN as u64)
        .is_none_or(|end| end > source.len())
    {
        return Err(FormatError::corrupt(
            offset,
            "ZIP64 end record lies outside the file",
        ));
    }
    let bytes = source.read_at(offset, ZIP64_END_LEN)?;
    let mut reader = ByteReader::with_base(&bytes, offset);
    if reader.read_u32()? != ZIP64_END_SIGNATURE {
        return Err(FormatError::corrupt(
            offset,
            "ZIP64 end record signature missing",
        ));
    }
    reader.skip(ZIP64_END_SIZE_VERSIONS_DISKS_LEN)?;
    let entry_count = reader.read_u64()?;
    let size = reader.read_u64()?;
    let directory_offset = reader.read_u64()?;
    if directory_offset
        .checked_add(size)
        .is_none_or(|end| end > offset)
    {
        return Err(FormatError::corrupt(
            offset,
            "central directory lies outside the file",
        ));
    }
    Ok(DirectoryLocation {
        entry_count,
        offset: directory_offset,
        size,
    })
}

fn find_end_record(tail: &[u8]) -> Option<usize> {
    let signature = END_SIGNATURE.to_le_bytes();
    let last_start = tail.len().checked_sub(END_LEN)?;
    (0..=last_start)
        .rev()
        .find(|&index| tail.get(index..index + signature.len()) == Some(signature.as_slice()))
}
