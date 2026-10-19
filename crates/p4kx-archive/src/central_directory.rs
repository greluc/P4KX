use p4kx_foundation::{ByteReader, ParseError};

use crate::entry::DataStart;
use crate::format_error::FormatError;
use crate::issue::{Directory, Issues, decode_path};
use crate::{Checksum, Compression, Entry, EntryIssue};

const CENTRAL_HEADER_SIGNATURE: u32 = 0x0201_4B50;
const CENTRAL_HEADER_LEN: usize = 46;
const VERSIONS_AND_FLAGS_LEN: usize = 6;
const TIME_AND_DATE_LEN: usize = 4;
const DISK_AND_ATTRIBUTES_LEN: usize = 8;
const ZIP64_MARKER_32: u32 = 0xFFFF_FFFF;
const BLOCK_ZIP64: u16 = 0x0001;
const BLOCK_CIG_ENCRYPTION: u16 = 0x5002;
const BLOCK_HEADER_LEN: usize = 4;
const ENCRYPTION_BLOCK_LEN: usize = 6;

struct FixedFields {
    method: u16,
    crc32c: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    local_header_offset: u32,
    name_len: usize,
    extra_len: usize,
    comment_len: usize,
}

struct ExtraFields {
    compressed_size: u64,
    uncompressed_size: u64,
    local_header_offset: u64,
    encrypted: bool,
}

/// Parses `count` Entries from the central directory bytes, which start at file offset `base`.
pub(crate) fn parse_entries(bytes: &[u8], base: u64, count: u64) -> Result<Directory, FormatError> {
    let mut reader = ByteReader::with_base(bytes, base);
    let capacity = usize::try_from(count)
        .unwrap_or(0)
        .min(bytes.len() / CENTRAL_HEADER_LEN);
    let mut entries = Vec::with_capacity(capacity);
    let mut issues = Issues::new();
    for _ in 0..count {
        let entry = parse_entry(&mut reader, entries.len(), &mut issues)?;
        entries.push(entry);
    }
    Ok(Directory { entries, issues })
}

fn parse_entry(
    reader: &mut ByteReader<'_>,
    index: usize,
    issues: &mut Issues,
) -> Result<Entry, FormatError> {
    let offset = reader.offset();
    let fixed = read_fixed_fields(reader, offset)?;
    let name = reader.read_bytes(fixed.name_len)?;
    let path = decode_path(name, index, issues);
    let extra_offset = reader.offset();
    let extra = reader.read_bytes(fixed.extra_len)?;
    reader.skip(fixed.comment_len)?;
    let extra = parse_extra(extra, extra_offset, &fixed, index, issues)?;
    Ok(Entry {
        path,
        compressed_size: extra.compressed_size,
        uncompressed_size: extra.uncompressed_size,
        checksum: Checksum::Crc32c(fixed.crc32c),
        data: DataStart::LocalHeader(extra.local_header_offset),
        compression: Compression::from_method(fixed.method),
        encrypted: extra.encrypted,
    })
}

fn read_fixed_fields(reader: &mut ByteReader<'_>, offset: u64) -> Result<FixedFields, FormatError> {
    if reader.read_u32()? != CENTRAL_HEADER_SIGNATURE {
        return Err(FormatError::corrupt(
            offset,
            "central directory header signature missing",
        ));
    }
    reader.skip(VERSIONS_AND_FLAGS_LEN)?;
    let method = reader.read_u16()?;
    reader.skip(TIME_AND_DATE_LEN)?;
    let crc32c = reader.read_u32()?;
    let compressed_size = reader.read_u32()?;
    let uncompressed_size = reader.read_u32()?;
    let name_len = usize::from(reader.read_u16()?);
    let extra_len = usize::from(reader.read_u16()?);
    let comment_len = usize::from(reader.read_u16()?);
    reader.skip(DISK_AND_ATTRIBUTES_LEN)?;
    let local_header_offset = reader.read_u32()?;
    Ok(FixedFields {
        method,
        crc32c,
        compressed_size,
        uncompressed_size,
        local_header_offset,
        name_len,
        extra_len,
        comment_len,
    })
}

fn parse_extra(
    extra: &[u8],
    base: u64,
    fixed: &FixedFields,
    index: usize,
    issues: &mut Issues,
) -> Result<ExtraFields, FormatError> {
    let mut fields = ExtraFields {
        compressed_size: u64::from(fixed.compressed_size),
        uncompressed_size: u64::from(fixed.uncompressed_size),
        local_header_offset: u64::from(fixed.local_header_offset),
        encrypted: false,
    };
    let mut reader = ByteReader::with_base(extra, base);
    while reader.remaining() >= BLOCK_HEADER_LEN {
        let block_offset = reader.offset();
        let id = reader.read_u16()?;
        let stored_len = reader.read_u16()?;
        let len = usize::from(stored_len);
        let data_len = len.checked_sub(BLOCK_HEADER_LEN).ok_or_else(|| {
            FormatError::corrupt(
                block_offset,
                format!("extra block 0x{id:04X} is shorter than its header"),
            )
        })?;
        let data = reader.read_bytes(data_len)?;
        let mut block = ByteReader::with_base(data, block_offset + BLOCK_HEADER_LEN as u64);
        match id {
            BLOCK_ZIP64 => read_zip64(&mut block, fixed, &mut fields)?,
            BLOCK_CIG_ENCRYPTION => {
                fields.encrypted = read_encryption(&mut block, stored_len, index, issues);
            }
            _ => {}
        }
    }
    if reader.remaining() > 0 {
        return Err(FormatError::corrupt(
            reader.offset(),
            "extra field ends inside a block header",
        ));
    }
    Ok(fields)
}

/// Reads the CIG encryption block: 0 is plain and 1 encrypted. Any other value and a block of
/// another length than 6 bytes are recorded in `issues`; a value that is not 0 or 1, and a block
/// too short to hold one, count as encrypted.
fn read_encryption(
    block: &mut ByteReader<'_>,
    stored_len: u16,
    index: usize,
    issues: &mut Issues,
) -> bool {
    if usize::from(stored_len) != ENCRYPTION_BLOCK_LEN {
        issues.push((index, EntryIssue::EncryptionBlockSize { size: stored_len }));
    }
    match block.read_u16() {
        Ok(0) => false,
        Ok(1) | Err(_) => true,
        Ok(value) => {
            issues.push((index, EntryIssue::UnknownEncryption { value }));
            true
        }
    }
}

fn read_zip64(
    block: &mut ByteReader<'_>,
    fixed: &FixedFields,
    fields: &mut ExtraFields,
) -> Result<(), ParseError> {
    if fixed.uncompressed_size == ZIP64_MARKER_32 {
        fields.uncompressed_size = block.read_u64()?;
    }
    if fixed.compressed_size == ZIP64_MARKER_32 {
        fields.compressed_size = block.read_u64()?;
    }
    if fixed.local_header_offset == ZIP64_MARKER_32 {
        fields.local_header_offset = block.read_u64()?;
    }
    Ok(())
}
