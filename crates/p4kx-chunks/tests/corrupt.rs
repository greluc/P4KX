//! Corrupt and truncated chunk files give a typed error, never a panic.

use p4kx_chunks::{ChunkError, ChunkFile, Container};
use p4kx_testkit::TestChunkFile;

fn parse(bytes: &[u8]) -> Result<ChunkFile<&[u8]>, ChunkError> {
    ChunkFile::parse(bytes)
}

fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
    if let Some(slot) = bytes.get_mut(at..at + 4) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

fn put_u64(bytes: &mut [u8], at: usize, value: u64) {
    if let Some(slot) = bytes.get_mut(at..at + 8) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

#[test]
fn another_signature_is_rejected() {
    let result = parse(b"DDS \x7c\0\0\0\0\0\0\0\0\0\0\0");

    assert!(
        matches!(result, Err(ChunkError::UnknownSignature { signature }) if &signature == b"DDS "),
        "{result:?}"
    );
}

#[test]
fn another_version_is_rejected() {
    let mut bytes = TestChunkFile::crch().bytes();
    put_u32(&mut bytes, 4, 0x745);

    let result = parse(&bytes);

    assert!(
        matches!(
            result,
            Err(ChunkError::UnsupportedVersion {
                container: Container::CrCh,
                version: 0x745
            })
        ),
        "{result:?}"
    );
}

#[test]
fn every_truncation_of_both_containers_is_an_error() {
    let files = [
        TestChunkFile::ivo()
            .chunk(0x9293_B9D8, 0x900, [1; 16])
            .chunk(0x8335_674E, 0x900, [2; 8])
            .bytes(),
        TestChunkFile::crch()
            .chunk(0x100B, 0x824, [1; 16])
            .big_endian_chunk(0x1016, 0x800, [2; 8])
            .bytes(),
    ];
    for bytes in &files {
        let first_data = 16 + 2 * 16;
        for len in 0..first_data {
            let prefix = bytes.get(..len).unwrap_or_default();
            assert!(parse(prefix).is_err(), "prefix of {len} bytes");
        }
    }
}

#[test]
fn a_truncated_crch_chunk_is_out_of_bounds() {
    let bytes = TestChunkFile::crch().chunk(0x100B, 0x824, [1; 16]).bytes();
    let truncated = bytes.get(..bytes.len() - 1).unwrap_or_default();

    let result = parse(truncated);

    assert!(
        matches!(
            result,
            Err(ChunkError::ChunkOutOfBounds {
                index: 0,
                size: 16,
                ..
            })
        ),
        "{result:?}"
    );
}

#[test]
fn a_chunk_table_past_the_end_is_rejected() {
    let mut bytes = TestChunkFile::ivo()
        .chunk(0x9293_B9D8, 0x900, [1; 8])
        .bytes();
    put_u32(&mut bytes, 8, u32::MAX);

    let result = parse(&bytes);

    assert!(
        matches!(
            result,
            Err(ChunkError::TableOutOfBounds {
                count: u32::MAX,
                ..
            })
        ),
        "{result:?}"
    );
}

#[test]
fn a_chunk_table_inside_the_header_is_rejected() {
    let mut bytes = TestChunkFile::crch().chunk(0x100B, 0x824, [1; 8]).bytes();
    put_u32(&mut bytes, 12, 4);

    let result = parse(&bytes);

    assert!(
        matches!(result, Err(ChunkError::TableOutOfBounds { offset: 4, .. })),
        "{result:?}"
    );
}

#[test]
fn an_ivo_offset_past_the_end_is_rejected() {
    let mut bytes = TestChunkFile::ivo()
        .chunk(0x9293_B9D8, 0x900, [1; 8])
        .bytes();
    put_u64(&mut bytes, 24, u64::MAX);

    let result = parse(&bytes);

    assert!(
        matches!(
            result,
            Err(ChunkError::ChunkOutOfBounds {
                index: 0,
                offset: u64::MAX,
                ..
            })
        ),
        "{result:?}"
    );
}

#[test]
fn a_crch_offset_and_size_that_overflow_are_rejected() {
    let mut bytes = TestChunkFile::crch().chunk(0x100B, 0x824, [1; 8]).bytes();
    put_u32(&mut bytes, 24, u32::MAX);
    put_u32(&mut bytes, 28, u32::MAX);

    let result = parse(&bytes);

    assert!(
        matches!(result, Err(ChunkError::ChunkOutOfBounds { index: 0, .. })),
        "{result:?}"
    );
}

#[test]
fn errors_name_what_is_wrong() {
    let message = parse(b"abcd")
        .map(|_| ())
        .err()
        .map(|error| error.to_string());

    assert_eq!(
        message.as_deref(),
        Some("not a chunk file: the signature is \"abcd\", not \"#ivo\" or \"CrCh\""),
        "message"
    );
}
