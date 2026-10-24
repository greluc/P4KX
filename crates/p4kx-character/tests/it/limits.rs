// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The caps that keep corrupt CHF files from sizing allocations or the zstd window.

use std::error::Error;

use p4kx_character::{CHF_SIZE, Chf, ChfError, MAX_FACE_PARTS};
use p4kx_testkit::TestChf;
use proptest::collection::vec;
use proptest::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];

fn chf_with_frame(frame: &[u8], body_len: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(CHF_SIZE);
    out.extend_from_slice(&0x4242_u16.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&u32::try_from(frame.len()).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(&body_len.to_le_bytes());
    out.extend_from_slice(frame);
    out.resize(CHF_SIZE, 0);
    with_crc(out)
}

fn with_crc(mut bytes: Vec<u8>) -> Vec<u8> {
    let crc = crc32c::crc32c(bytes.get(16..).unwrap_or_default());
    if let Some(slot) = bytes.get_mut(4..8) {
        slot.copy_from_slice(&crc.to_le_bytes());
    }
    bytes
}

#[test]
fn an_rle_frame_that_yields_more_than_the_stated_body_is_a_size_mismatch() {
    let mut frame = ZSTD_MAGIC.to_vec();
    frame.extend_from_slice(&[0x20, 100]);
    frame.extend_from_slice(&[0x23, 0x03, 0x00, b'A']);

    let error = Chf::read(&chf_with_frame(&frame, 10));

    assert!(
        matches!(
            error,
            Err(ChfError::BodySizeMismatch {
                stored: 10,
                actual: 11
            })
        ),
        "{error:?}"
    );
}

#[test]
fn a_frame_with_a_window_above_one_mebibyte_is_corrupt() {
    let mut frame = ZSTD_MAGIC.to_vec();
    frame.extend_from_slice(&[0x00, 11 << 3]);
    frame.extend_from_slice(&[0x09, 0x00, 0x00, b'A']);

    let error = Chf::read(&chf_with_frame(&frame, 1));

    assert!(
        matches!(error, Err(ChfError::CorruptFrame { .. })),
        "{error:?}"
    );
}

#[test]
fn a_dna_block_with_too_many_face_parts_is_rejected_on_read_and_write() -> TestResult {
    let parts = MAX_FACE_PARTS + 1;
    let test = TestChf {
        face_parts: vec![Vec::new(); parts],
        ..TestChf::v7()
    };
    let mut chf = Chf::read(&TestChf::v7().to_bytes())?;
    if let Some(dna) = &mut chf.character.dna {
        dna.blends_per_part = 0;
        dna.face_parts = vec![Vec::new(); parts];
    }

    let read = Chf::read(&test.to_bytes());

    assert!(
        matches!(read, Err(ChfError::UnexpectedValue { found, .. }) if found == parts as u128),
        "read: {read:?}"
    );
    assert!(
        matches!(chf.write(), Err(ChfError::Unwritable { .. })),
        "write"
    );
    Ok(())
}

fn base_files() -> Result<Vec<Vec<u8>>, ChfError> {
    let compressed = Chf::new(Chf::read(&TestChf::v8().to_bytes())?.character).write()?;
    Ok(vec![TestChf::v7().to_bytes(), compressed])
}

proptest! {
    #[test]
    fn a_corrupt_frame_behind_a_valid_crc_never_panics_and_round_trips_when_accepted(
        base in 0..2_usize,
        edits in vec((any::<prop::sample::Index>(), any::<u8>()), 1..=6),
    ) {
        let files = base_files().map_err(|error| TestCaseError::fail(error.to_string()))?;
        let mut bytes = files.get(base).cloned().unwrap_or_default();
        let frame_len = bytes
            .get(8..12)
            .and_then(|len| <[u8; 4]>::try_from(len).ok())
            .map_or(0, |len| u32::from_le_bytes(len) as usize);
        for (at, value) in edits {
            if let Some(byte) = bytes.get_mut(16 + at.index(frame_len)) {
                *byte = value;
            }
        }
        let bytes = with_crc(bytes);

        let read = Chf::read(&bytes);

        if let Ok(chf) = read {
            prop_assert_eq!(chf.write().ok(), Some(bytes));
        }
    }
}
