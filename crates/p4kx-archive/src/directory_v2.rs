use p4kx_foundation::ByteReader;

use crate::entry::DataStart;
use crate::format_error::FormatError;
use crate::issue::{Directory, Issues, decode_path};
use crate::source::Source;
use crate::{Checksum, Compression, Entry, EntryIssue, layout};

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

/// Reads the Entries of a v2 Archive from its trailer, directory records and name table, refusing
/// more than `max_entries` before the directory is read.
pub(crate) fn read_entries(
    source: &Source,
    max_entries: Option<u64>,
) -> Result<Directory, FormatError> {
    let trailer_offset = source
        .len()
        .checked_sub(TRAILER_LEN as u64)
        .ok_or_else(|| FormatError::corrupt(0, "v2 trailer is cut off"))?;
    let trailer = read_trailer(source, trailer_offset, max_entries)?;
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
    let mut issues = Issues::new();
    let mut record_offset = trailer.directory_offset;
    for record in directory.as_chunks::<RECORD_LEN>().0 {
        let entry = parse_record(
            record,
            record_offset,
            &names,
            (entries.len(), trailer_offset),
            &mut issues,
        )?;
        entries.push(entry);
        record_offset += RECORD_LEN as u64;
    }
    Ok(Directory { entries, issues })
}

fn read_trailer(
    source: &Source,
    offset: u64,
    max_entries: Option<u64>,
) -> Result<Trailer, FormatError> {
    let bytes = source.read_at(offset, TRAILER_LEN)?;
    let mut reader = ByteReader::with_base(&bytes, offset);
    let entry_count = reader.read_u64()?;
    reader.skip(TRAILER_EXTENSION_COUNT_LEN)?;
    let directory_offset = reader.read_u64()?;
    let directory_size = reader.read_u64()?;
    reader.skip(TRAILER_EXTENSION_SIZE_LEN)?;
    let names_offset = reader.read_u64()?;
    let names_size = reader.read_u64()?;
    layout::check_entry_count(entry_count, max_entries)?;
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

/// Parses the record at file `offset`; `at` is the Entry's position and the offset of the trailer,
/// which its data must end before.
fn parse_record(
    record: &[u8],
    offset: u64,
    names: &[u8],
    at: (usize, u64),
    issues: &mut Issues,
) -> Result<Entry, FormatError> {
    let (index, trailer_offset) = at;
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
    if data_offset
        .checked_add(compressed_size)
        .is_none_or(|end| end > trailer_offset)
    {
        issues.push((index, EntryIssue::DataOutsideFile));
    }
    if encryption > 1 {
        issues.push((index, EntryIssue::UnknownEncryption { value: encryption }));
    }
    Ok(Entry {
        path: read_name(names, name_offset, offset, index, issues)?,
        compressed_size,
        uncompressed_size,
        checksum: Checksum::Crc32c(crc32c),
        data: DataStart::Direct(data_offset),
        compression: Compression::from_method(method),
        encrypted: encryption != 0,
    })
}

fn read_name(
    names: &[u8],
    name_offset: u64,
    record_offset: u64,
    index: usize,
    issues: &mut Issues,
) -> Result<String, FormatError> {
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
    let name = name.get(..end).unwrap_or_default();
    if name.is_empty() {
        return Err(FormatError::corrupt(record_offset, "Entry path is empty"));
    }
    Ok(decode_path(name, index, issues).replace('/', "\\"))
}
