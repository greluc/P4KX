// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `ChunkFile::parse` never panics, and every chunk of a file it accepts can be read whole.

use p4kx_chunks::ChunkFile;
use p4kx_testkit::TestChunkFile;
use proptest::collection::vec;
use proptest::prelude::*;

fn check(bytes: &[u8]) -> Result<(), TestCaseError> {
    if let Ok(file) = ChunkFile::parse(bytes) {
        for (index, chunk) in file.chunks().iter().enumerate() {
            let len = file.chunk_data(index).map(<[u8]>::len);
            prop_assert_eq!(len, usize::try_from(chunk.size()).ok(), "chunk {}", index);
        }
    }
    Ok(())
}

fn fixture(ivo: bool, chunks: &[(u32, Vec<u8>)], big_endian: bool, table_at_end: bool) -> Vec<u8> {
    let mut file = if ivo {
        TestChunkFile::ivo()
    } else {
        TestChunkFile::crch()
    };
    for (type_id, data) in chunks {
        file = if big_endian {
            file.big_endian_chunk(*type_id, 0x800, data.clone())
        } else {
            file.chunk(*type_id, 0x900, data.clone())
        };
    }
    if table_at_end {
        file = file.table_at_end();
    }
    file.bytes()
}

proptest! {
    #[test]
    fn arbitrary_bytes_never_panic(bytes in vec(any::<u8>(), 0..512)) {
        check(&bytes)?;
    }

    #[test]
    fn arbitrary_bytes_behind_a_valid_header_never_panic(
        ivo in any::<bool>(),
        header in (any::<u32>(), 0_u32..64),
        rest in vec(any::<u8>(), 0..512),
    ) {
        let mut bytes = if ivo { b"#ivo".to_vec() } else { b"CrCh".to_vec() };
        bytes.extend_from_slice(&(if ivo { 0x900_u32 } else { 0x746 }).to_le_bytes());
        bytes.extend_from_slice(&header.1.to_le_bytes());
        bytes.extend_from_slice(&(header.0 % 600).to_le_bytes());
        bytes.extend_from_slice(&rest);
        check(&bytes)?;
    }

    #[test]
    fn mutated_chunk_files_never_panic(
        ivo in any::<bool>(),
        chunks in vec((any::<u32>(), vec(any::<u8>(), 0..40)), 0..6),
        big_endian in any::<bool>(),
        table_at_end in any::<bool>(),
        flips in vec((any::<usize>(), any::<u8>()), 0..8),
        cut in any::<usize>(),
    ) {
        let mut bytes = fixture(ivo, &chunks, big_endian, table_at_end);
        let len = bytes.len();
        if len > 0 {
            for (at, value) in flips {
                if let Some(byte) = bytes.get_mut(at % len) {
                    *byte = value;
                }
            }
            bytes.truncate(len - cut % len);
        }
        check(&bytes)?;
    }

    #[test]
    fn unchanged_chunk_files_parse(
        ivo in any::<bool>(),
        chunks in vec((any::<u32>(), vec(any::<u8>(), 0..40)), 0..6),
        table_at_end in any::<bool>(),
    ) {
        let bytes = fixture(ivo, &chunks, false, table_at_end);
        let file = ChunkFile::parse(bytes.as_slice());
        prop_assert!(file.is_ok(), "{:?}", file.err());
        check(&bytes)?;
    }
}
