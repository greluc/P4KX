use p4kx_foundation::ByteReader;

use crate::entry::DataStart;
use crate::format_error::FormatError;
use crate::source::Source;
use crate::{Compression, Entry};

const TRAILER_LEN: usize = 175;
const RECORD_LEN: usize = 204;
const TRAILER_EXTENSION_COUNT_LEN: usize = 8;
const TRAILER_EXTENSION_SIZE_LEN: usize = 8;
const RECORD_TIME_AND_DATE_LEN: usize = 4;
const RECORD_SIGNATURE_LEN: usize = 128;

struct Trailer {
    directory_offset: u64,
    directory_size: u64,
    names_offset: u64,
    names_size: u64,
}

/// Reads the Entries of a v2 Archive from its trailer, directory records and name table.
pub(crate) fn read_entries(source: &Source) -> Result<Vec<Entry>, FormatError> {
    let trailer_offset = source
        .len()
        .checked_sub(TRAILER_LEN as u64)
        .ok_or_else(|| FormatError::corrupt(0, "v2 trailer is cut off"))?;
    let trailer = read_trailer(source, trailer_offset)?;
    let directory = read_range(
        source,
        trailer.directory_offset,
        trailer.directory_size,
        trailer_offset,
        "central directory",
    )?;
    let names = read_range(
        source,
        trailer.names_offset,
        trailer.names_size,
        trailer_offset,
        "name table",
    )?;
    let mut entries = Vec::with_capacity(directory.len() / RECORD_LEN);
    let mut record_offset = trailer.directory_offset;
    for record in directory.as_chunks::<RECORD_LEN>().0 {
        entries.push(parse_record(record, record_offset, &names)?);
        record_offset += RECORD_LEN as u64;
    }
    Ok(entries)
}

fn read_trailer(source: &Source, offset: u64) -> Result<Trailer, FormatError> {
    let bytes = source.read_at(offset, TRAILER_LEN)?;
    let mut reader = ByteReader::with_base(&bytes, offset);
    let entry_count = reader.read_u64()?;
    reader.skip(TRAILER_EXTENSION_COUNT_LEN)?;
    let directory_offset = reader.read_u64()?;
    let directory_size = reader.read_u64()?;
    reader.skip(TRAILER_EXTENSION_SIZE_LEN)?;
    let names_offset = reader.read_u64()?;
    let names_size = reader.read_u64()?;
    if entry_count.checked_mul(RECORD_LEN as u64) != Some(directory_size) {
        return Err(FormatError::corrupt(
            offset,
            format!("central directory size does not match its {entry_count} Entries"),
        ));
    }
    Ok(Trailer {
        directory_offset,
        directory_size,
        names_offset,
        names_size,
    })
}

fn read_range(
    source: &Source,
    offset: u64,
    size: u64,
    trailer_offset: u64,
    what: &str,
) -> Result<Vec<u8>, FormatError> {
    if offset
        .checked_add(size)
        .is_none_or(|end| end > trailer_offset)
    {
        return Err(FormatError::corrupt(
            trailer_offset,
            format!("{what} lies outside the file"),
        ));
    }
    let size = usize::try_from(size)
        .map_err(|_| FormatError::corrupt(offset, format!("{what} does not fit into memory")))?;
    Ok(source.read_at(offset, size)?)
}

fn parse_record(record: &[u8], offset: u64, names: &[u8]) -> Result<Entry, FormatError> {
    let mut reader = ByteReader::with_base(record, offset);
    let method = reader.read_u16()?;
    reader.skip(RECORD_TIME_AND_DATE_LEN)?;
    let crc32c = reader.read_u32()?;
    let compressed_size = reader.read_u64()?;
    let uncompressed_size = reader.read_u64()?;
    let data_offset = reader.read_u64()?;
    let name_offset = reader.read_u64()?;
    reader.skip(RECORD_SIGNATURE_LEN)?;
    let encryption = reader.read_u16()?;
    Ok(Entry {
        path: read_name(names, name_offset, offset)?,
        compressed_size,
        uncompressed_size,
        crc32c,
        data: DataStart::Direct(data_offset),
        compression: Compression::from_method(method),
        encrypted: encryption != 0,
    })
}

fn read_name(names: &[u8], name_offset: u64, record_offset: u64) -> Result<String, FormatError> {
    let name = usize::try_from(name_offset)
        .ok()
        .and_then(|start| names.get(start..))
        .ok_or_else(|| {
            FormatError::corrupt(record_offset, "Entry path lies outside the name table")
        })?;
    let end = name
        .iter()
        .position(|&byte| byte == 0)
        .ok_or_else(|| FormatError::corrupt(record_offset, "Entry path is not terminated"))?;
    let path = std::str::from_utf8(name.get(..end).unwrap_or_default())
        .map_err(|_| FormatError::corrupt(record_offset, "Entry path is not UTF-8"))?;
    if path.is_empty() {
        return Err(FormatError::corrupt(record_offset, "Entry path is empty"));
    }
    Ok(path.replace('/', "\\"))
}
