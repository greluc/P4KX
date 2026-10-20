use crate::TestEntry;
use crate::bytes::{len_u16, len_u64, put_u16, put_u32, put_u64};

const LOCAL_HEADER_CIG: u32 = 0x1403_4B50;
const CENTRAL_HEADER: u32 = 0x0201_4B50;
const ZIP64_END: u32 = 0x0606_4B50;
const ZIP64_LOCATOR: u32 = 0x0706_4B50;
const END: u32 = 0x0605_4B50;
const VERSION: u16 = 45;
const MARKER_16: u16 = 0xFFFF;
const MARKER_32: u32 = 0xFFFF_FFFF;
const BLOCK_ZIP64: u16 = 0x0001;
const BLOCK_CIG_KEY: u16 = 0x5000;
const BLOCK_CIG_ENCRYPTION: u16 = 0x5002;
const BLOCK_CIG_HASH: u16 = 0x5003;
const CIG_KEY_LEN: usize = 128;
const END_COMMENT_LEN: u16 = 16;

/// Writes `entries` as a v1 Archive: local headers with data, the ZIP64 central directory and the
/// end records.
pub(crate) fn build(entries: &[TestEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    let offsets: Vec<u64> = entries
        .iter()
        .map(|entry| write_local_entry(&mut out, entry))
        .collect();
    let directory_offset = len_u64(&out);
    for (entry, offset) in entries.iter().zip(offsets) {
        write_central_entry(&mut out, entry, offset);
    }
    let directory_size = len_u64(&out) - directory_offset;
    write_end_records(&mut out, len_u64(entries), directory_offset, directory_size);
    out
}

fn write_local_entry(out: &mut Vec<u8>, entry: &TestEntry) -> u64 {
    let offset = len_u64(out);
    put_u32(out, LOCAL_HEADER_CIG);
    put_u16(out, VERSION);
    put_u16(out, 0);
    put_u16(out, entry.method);
    put_u32(out, 0);
    put_u32(out, entry.crc32c);
    put_u32(out, MARKER_32);
    put_u32(out, MARKER_32);
    put_u16(out, len_u16(entry.path_bytes()));
    put_u16(out, 0);
    out.extend_from_slice(entry.path_bytes());
    out.extend_from_slice(&entry.payload);
    offset
}

fn write_central_entry(out: &mut Vec<u8>, entry: &TestEntry, local_offset: u64) {
    let extra = central_extra(entry, local_offset);
    put_u32(out, CENTRAL_HEADER);
    put_u16(out, VERSION);
    put_u16(out, VERSION);
    put_u16(out, 0);
    put_u16(out, entry.method);
    put_u32(out, 0);
    put_u32(out, entry.crc32c);
    put_u32(out, MARKER_32);
    put_u32(out, MARKER_32);
    put_u16(out, len_u16(entry.path_bytes()));
    put_u16(out, len_u16(&extra));
    put_u16(out, 0);
    put_u16(out, MARKER_16);
    put_u16(out, 0);
    put_u32(out, 0);
    put_u32(out, MARKER_32);
    out.extend_from_slice(entry.path_bytes());
    out.extend_from_slice(&extra);
}

fn central_extra(entry: &TestEntry, local_offset: u64) -> Vec<u8> {
    let mut zip64 = Vec::new();
    put_u64(&mut zip64, entry.uncompressed_size);
    put_u64(&mut zip64, len_u64(&entry.payload));
    put_u64(&mut zip64, local_offset);
    put_u32(&mut zip64, 0);
    let mut extra = Vec::new();
    put_block(&mut extra, BLOCK_ZIP64, &zip64);
    put_block(&mut extra, BLOCK_CIG_KEY, &[0; CIG_KEY_LEN]);
    put_block(
        &mut extra,
        BLOCK_CIG_ENCRYPTION,
        &entry
            .encryption_block
            .clone()
            .unwrap_or_else(|| entry.encryption_value().to_le_bytes().to_vec()),
    );
    put_block(&mut extra, BLOCK_CIG_HASH, &entry.sha256);
    extra.extend_from_slice(&entry.extra_blocks);
    extra
}

fn write_end_records(out: &mut Vec<u8>, count: u64, directory_offset: u64, directory_size: u64) {
    let zip64_end_offset = len_u64(out);
    put_u32(out, ZIP64_END);
    put_u64(out, 44);
    put_u16(out, 46);
    put_u16(out, VERSION);
    put_u32(out, 0);
    put_u32(out, 0);
    put_u64(out, count);
    put_u64(out, count);
    put_u64(out, directory_size);
    put_u64(out, directory_offset);
    put_u32(out, ZIP64_LOCATOR);
    put_u32(out, 0);
    put_u64(out, zip64_end_offset);
    put_u32(out, 1);
    put_u32(out, END);
    for _ in 0..4 {
        put_u16(out, MARKER_16);
    }
    put_u32(out, MARKER_32);
    put_u32(out, MARKER_32);
    put_u16(out, END_COMMENT_LEN);
    out.extend_from_slice(&[0; END_COMMENT_LEN as usize]);
}

fn put_block(out: &mut Vec<u8>, id: u16, data: &[u8]) {
    put_u16(out, id);
    put_u16(out, len_u16(data) + 4);
    out.extend_from_slice(data);
}
