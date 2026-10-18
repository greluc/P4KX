use p4kx_foundation::{ByteReader, ParseError};

use crate::{Compression, Entry};

const CENTRAL_HEADER: u32 = 0x0201_4B50;
const MARKER_32: u32 = 0xFFFF_FFFF;
const BLOCK_ZIP64: u16 = 0x0001;
const BLOCK_CIG_ENCRYPTION: u16 = 0x5002;
const BLOCK_HEADER_LEN: usize = 4;

/// Why the central directory could not be parsed.
#[derive(Debug)]
pub(crate) enum DirectoryError {
    Corrupt { offset: u64, reason: String },
    Truncated(ParseError),
}

impl From<ParseError> for DirectoryError {
    fn from(error: ParseError) -> Self {
        Self::Truncated(error)
    }
}

struct FixedFields {
    method: u16,
    compressed_size: u32,
    uncompressed_size: u32,
    name_len: usize,
    extra_len: usize,
    comment_len: usize,
}

/// Parses `count` Entries from the central directory bytes, which start at file offset `base`.
pub(crate) fn parse_entries(
    bytes: &[u8],
    base: u64,
    count: u64,
) -> Result<Vec<Entry>, DirectoryError> {
    let mut reader = ByteReader::with_base(bytes, base);
    let capacity = usize::try_from(count).unwrap_or(0).min(bytes.len() / 46);
    let mut entries = Vec::with_capacity(capacity);
    for _ in 0..count {
        entries.push(parse_entry(&mut reader)?);
    }
    Ok(entries)
}

fn parse_entry(reader: &mut ByteReader<'_>) -> Result<Entry, DirectoryError> {
    let offset = reader.offset();
    let fixed = read_fixed_fields(reader, offset)?;
    let name = reader.read_bytes(fixed.name_len)?;
    let path = String::from_utf8(name.to_vec()).map_err(|_| DirectoryError::Corrupt {
        offset,
        reason: "Entry path is not UTF-8".to_owned(),
    })?;
    let extra_offset = reader.offset();
    let extra = reader.read_bytes(fixed.extra_len)?;
    reader.skip(fixed.comment_len)?;
    let extra = parse_extra(extra, extra_offset, &fixed)?;
    Ok(Entry {
        path,
        compressed_size: extra.compressed_size,
        uncompressed_size: extra.uncompressed_size,
        compression: Compression::from_method(fixed.method),
        encrypted: extra.encrypted,
    })
}

fn read_fixed_fields(
    reader: &mut ByteReader<'_>,
    offset: u64,
) -> Result<FixedFields, DirectoryError> {
    if reader.read_u32()? != CENTRAL_HEADER {
        return Err(DirectoryError::Corrupt {
            offset,
            reason: "central directory header signature missing".to_owned(),
        });
    }
    reader.skip(6)?;
    let method = reader.read_u16()?;
    reader.skip(8)?;
    let compressed_size = reader.read_u32()?;
    let uncompressed_size = reader.read_u32()?;
    let name_len = usize::from(reader.read_u16()?);
    let extra_len = usize::from(reader.read_u16()?);
    let comment_len = usize::from(reader.read_u16()?);
    reader.skip(12)?;
    Ok(FixedFields {
        method,
        compressed_size,
        uncompressed_size,
        name_len,
        extra_len,
        comment_len,
    })
}

struct ExtraFields {
    compressed_size: u64,
    uncompressed_size: u64,
    encrypted: bool,
}

fn parse_extra(
    extra: &[u8],
    base: u64,
    fixed: &FixedFields,
) -> Result<ExtraFields, DirectoryError> {
    let mut fields = ExtraFields {
        compressed_size: u64::from(fixed.compressed_size),
        uncompressed_size: u64::from(fixed.uncompressed_size),
        encrypted: false,
    };
    let mut reader = ByteReader::with_base(extra, base);
    while reader.remaining() >= BLOCK_HEADER_LEN {
        let block_offset = reader.offset();
        let id = reader.read_u16()?;
        let len = usize::from(reader.read_u16()?);
        let data_len =
            len.checked_sub(BLOCK_HEADER_LEN)
                .ok_or_else(|| DirectoryError::Corrupt {
                    offset: block_offset,
                    reason: format!("extra block 0x{id:04X} is shorter than its header"),
                })?;
        let data = reader.read_bytes(data_len)?;
        let mut block = ByteReader::with_base(data, block_offset + BLOCK_HEADER_LEN as u64);
        match id {
            BLOCK_ZIP64 => read_zip64(&mut block, fixed, &mut fields)?,
            BLOCK_CIG_ENCRYPTION => fields.encrypted = block.read_u16()? != 0,
            _ => {}
        }
    }
    Ok(fields)
}

fn read_zip64(
    block: &mut ByteReader<'_>,
    fixed: &FixedFields,
    fields: &mut ExtraFields,
) -> Result<(), ParseError> {
    if fixed.uncompressed_size == MARKER_32 {
        fields.uncompressed_size = block.read_u64()?;
    }
    if fixed.compressed_size == MARKER_32 {
        fields.compressed_size = block.read_u64()?;
    }
    Ok(())
}
