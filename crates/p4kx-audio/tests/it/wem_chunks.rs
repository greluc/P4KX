// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Every RIFF chunk of a WEM is indexed, wherever it lies.

use p4kx_audio::{AudioError, Wem};
use p4kx_testkit::{TestCodec, TestWem, test_signal};

type TestResult = Result<(), AudioError>;

fn wem() -> TestWem {
    TestWem::pcm(TestCodec::WwisePcm, 1, 48_000, &test_signal(64, 1))
}

#[test]
fn chunks_after_data_are_found_in_file_order() -> TestResult {
    let bytes = wem()
        .chunk(*b"junk", [1, 2])
        .tail_chunk(*b"smpl", [7; 36])
        .tail_chunk(*b"cue ", [5; 5])
        .tail_chunk(*b"LIST", [])
        .bytes();

    let wem = Wem::parse(bytes.as_slice())?;

    let ids: Vec<[u8; 4]> = wem.chunks().map(|chunk| chunk.id).collect();
    assert_eq!(
        ids,
        [*b"fmt ", *b"junk", *b"data", *b"smpl", *b"cue ", *b"LIST"]
    );
    assert_eq!(wem.chunk(*b"smpl"), Some([7; 36].as_slice()), "smpl");
    assert_eq!(wem.chunk(*b"cue "), Some([5; 5].as_slice()), "odd body");
    assert_eq!(wem.chunk(*b"LIST"), Some([].as_slice()), "empty body");
    assert_eq!(wem.chunk(*b"nope"), None, "absent");
    assert_eq!(wem.data().len(), 128, "data is unchanged");
    Ok(())
}

#[test]
fn a_chunk_after_data_that_runs_past_the_file_ends_the_list_not_the_wem() -> TestResult {
    let mut bytes = wem().tail_chunk(*b"smpl", [7; 4]).bytes();
    let size_at = bytes.len() - 8;
    if let Some(size) = bytes.get_mut(size_at..size_at + 4) {
        size.copy_from_slice(&1000u32.to_le_bytes());
    }

    let wem = Wem::parse(bytes.as_slice())?;

    let ids: Vec<[u8; 4]> = wem.chunks().map(|chunk| chunk.id).collect();
    assert_eq!(ids, [*b"fmt ", *b"data"], "the overlong chunk is dropped");
    assert_eq!(wem.chunk(*b"smpl"), None);
    Ok(())
}

#[test]
fn chunks_of_a_big_endian_wem_are_indexed_too() -> TestResult {
    let bytes = wem().big_endian().tail_chunk(*b"smpl", [1; 8]).bytes();

    let wem = Wem::parse(bytes.as_slice())?;

    assert_eq!(wem.chunk(*b"smpl"), Some([1; 8].as_slice()));
    Ok(())
}
