use p4kx_foundation::ByteReader;

use crate::format_error::FormatError;
use crate::source::Source;

const END_SIGNATURE: u32 = 0x0605_4B50;
const ZIP64_LOCATOR_SIGNATURE: u32 = 0x0706_4B50;
const ZIP64_END_SIGNATURE: u32 = 0x0606_4B50;
const END_LEN: usize = 22;
const END_DISKS_LEN: usize = 4;
const END_DISK_ENTRIES_LEN: usize = 2;
const MAX_COMMENT_LEN: usize = u16::MAX as usize;
const ZIP64_LOCATOR_LEN: usize = 20;
const ZIP64_LOCATOR_DISK_LEN: usize = 4;
const ZIP64_END_LEN: usize = 56;
const ZIP64_END_SIZE_VERSIONS_DISKS_LEN: usize = 28;
const MARKER_16: u16 = u16::MAX;
const MARKER_32: u32 = u32::MAX;

/// Where the central directory lies and how many Entries it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DirectoryLocation {
    pub(crate) entry_count: u64,
    pub(crate) offset: u64,
    pub(crate) size: u64,
    pub(crate) zip64: bool,
}

/// Reads the end-of-central-directory record and the ZIP64 records it points to, if any.
///
/// Without a ZIP64 locator before it, the end record itself gives the directory, as in the plain
/// ZIP archives of containers.
pub(crate) fn locate_directory(source: &Source) -> Result<DirectoryLocation, FormatError> {
    let tail_len = source.len().min((END_LEN + MAX_COMMENT_LEN) as u64);
    let tail_start = source.len() - tail_len;
    let tail = source.read_at(tail_start, usize::try_from(tail_len).unwrap_or(usize::MAX))?;
    let end_index = find_end_record(&tail).ok_or(FormatError::NotAnArchive)?;
    let end_offset = tail_start + end_index as u64;
    if let Some(zip64_end_offset) = find_zip64_locator(source, &tail, end_index, tail_start)? {
        return read_zip64_end(source, zip64_end_offset);
    }
    let end = tail.get(end_index..).unwrap_or_default();
    read_plain_end(end, end_offset)
}

/// Returns the offset of the ZIP64 end record the locator before the end record at `end_index` of
/// `tail` points to. The locator is expected right before the end record; when it is not, it is
/// searched backwards through the rest of `tail`, and only a locator whose target carries the
/// ZIP64 end record signature counts.
fn find_zip64_locator(
    source: &Source,
    tail: &[u8],
    end_index: usize,
    tail_start: u64,
) -> Result<Option<u64>, FormatError> {
    let end_offset = tail_start + end_index as u64;
    if let Some(locator_offset) = end_offset.checked_sub(ZIP64_LOCATOR_LEN as u64) {
        let locator = source.read_at(locator_offset, ZIP64_LOCATOR_LEN)?;
        if let Some(target) = locator_target(&locator, 0, locator_offset) {
            return Ok(Some(target));
        }
    }
    let Some(adjacent) = end_index.checked_sub(ZIP64_LOCATOR_LEN) else {
        return Ok(None);
    };
    let signature = ZIP64_LOCATOR_SIGNATURE.to_le_bytes();
    for start in (0..adjacent).rev() {
        if tail.get(start..start + signature.len()) != Some(signature.as_slice()) {
            continue;
        }
        let Some(target) = locator_target(tail, start, tail_start) else {
            continue;
        };
        let locator_offset = tail_start + start as u64;
        if target < locator_offset && has_zip64_end_signature(source, target)? {
            return Ok(Some(target));
        }
    }
    Ok(None)
}

fn locator_target(tail: &[u8], start: usize, tail_start: u64) -> Option<u64> {
    let locator = tail.get(start..start + ZIP64_LOCATOR_LEN)?;
    let mut reader = ByteReader::with_base(locator, tail_start + start as u64);
    if reader.read_u32().ok()? != ZIP64_LOCATOR_SIGNATURE {
        return None;
    }
    reader.skip(ZIP64_LOCATOR_DISK_LEN).ok()?;
    reader.read_u64().ok()
}

fn has_zip64_end_signature(source: &Source, offset: u64) -> Result<bool, FormatError> {
    let signature = ZIP64_END_SIGNATURE.to_le_bytes();
    if offset
        .checked_add(signature.len() as u64)
        .is_none_or(|end| end > source.len())
    {
        return Ok(false);
    }
    Ok(source.read_at(offset, signature.len())? == signature)
}

fn read_plain_end(end: &[u8], offset: u64) -> Result<DirectoryLocation, FormatError> {
    let mut reader = ByteReader::with_base(end, offset);
    reader.skip(END_SIGNATURE.to_le_bytes().len() + END_DISKS_LEN + END_DISK_ENTRIES_LEN)?;
    let entry_count = reader.read_u16()?;
    let size = reader.read_u32()?;
    let directory_offset = reader.read_u32()?;
    if entry_count == MARKER_16 || size == MARKER_32 || directory_offset == MARKER_32 {
        return Err(FormatError::corrupt(
            offset,
            "no ZIP64 locator before the end record",
        ));
    }
    let location = DirectoryLocation {
        entry_count: u64::from(entry_count),
        offset: u64::from(directory_offset),
        size: u64::from(size),
        zip64: false,
    };
    check_before(location, offset)
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
    let location = DirectoryLocation {
        entry_count: reader.read_u64()?,
        size: reader.read_u64()?,
        offset: reader.read_u64()?,
        zip64: true,
    };
    check_before(location, offset)
}

fn check_before(
    location: DirectoryLocation,
    record_offset: u64,
) -> Result<DirectoryLocation, FormatError> {
    if location
        .offset
        .checked_add(location.size)
        .is_none_or(|end| end > record_offset)
    {
        return Err(FormatError::corrupt(
            record_offset,
            "central directory lies outside the file",
        ));
    }
    Ok(location)
}

fn find_end_record(tail: &[u8]) -> Option<usize> {
    let signature = END_SIGNATURE.to_le_bytes();
    let last_start = tail.len().checked_sub(END_LEN)?;
    (0..=last_start)
        .rev()
        .find(|&index| tail.get(index..index + signature.len()) == Some(signature.as_slice()))
}
