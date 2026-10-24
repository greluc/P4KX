// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Decoding Wwise IMA ADPCM and PTADPCM WEMs to WAV, checked against hand-computed vectors.

use p4kx_audio::{AudioError, Wem};
use p4kx_testkit::{TestCodec, TestWem};

type TestResult = Result<(), AudioError>;

const IMA: TestCodec = TestCodec::Tag(0x0002);
const PTADPCM: TestCodec = TestCodec::Tag(0x8311);

fn wav_samples(wav: &[u8]) -> Vec<i16> {
    wav.get(44..)
        .unwrap_or_default()
        .as_chunks::<2>()
        .0
        .iter()
        .copied()
        .map(i16::from_le_bytes)
        .collect()
}

fn decode(wem: &TestWem) -> Result<Vec<i16>, AudioError> {
    let mut wav = Vec::new();
    Wem::parse(wem.bytes())?.write_wav(&mut wav)?;
    Ok(wav_samples(&wav))
}

/// A 36-byte IMA sub-block: the header sample, the step index, a reserved byte, then nibbles.
fn ima_block(header: [u8; 2], step_index: u8, nibbles: &[u8]) -> Vec<u8> {
    let mut block = vec![header[0], header[1], step_index, 0];
    block.extend_from_slice(nibbles);
    block.resize(0x24, 0);
    block
}

#[test]
fn a_mono_ima_block_decodes_to_the_header_sample_and_63_nibbles_low_nibble_first() -> TestResult {
    let wem = TestWem::encoded(IMA, 1, 48_000, 0)
        .block_align(0x24)
        .data(ima_block(100i16.to_le_bytes(), 0, &[0xF7, 0x08]));

    let samples = decode(&wem)?;

    assert_eq!(
        samples.len(),
        64,
        "64 samples per block, the last nibble is skipped"
    );
    assert_eq!(
        samples.get(..5),
        Some(&[100, 113, 83, 79, 82][..]),
        "the header sample, then nibbles 7, F, 8, 0 with the multiplying IMA expansion"
    );
    Ok(())
}

#[test]
fn stereo_ima_blocks_hold_one_sub_block_per_channel_and_decode_interleaved() -> TestResult {
    let mut data = ima_block(100i16.to_le_bytes(), 0, &[0xF7]);
    data.extend(ima_block((-200i16).to_le_bytes(), 0, &[0x07]));
    let wem = TestWem::encoded(IMA, 2, 48_000, 0)
        .block_align(0x48)
        .data(data);

    let samples = decode(&wem)?;

    assert_eq!(samples.len(), 2 * 64, "64 frames");
    assert_eq!(
        samples.get(..4),
        Some(&[100, -200, 113, -187][..]),
        "left and right alternate"
    );
    Ok(())
}

#[test]
fn a_big_endian_ima_wem_has_big_endian_block_headers() -> TestResult {
    let wem = TestWem::encoded(IMA, 1, 48_000, 0)
        .block_align(0x24)
        .data(ima_block(100i16.to_be_bytes(), 0, &[0xF7, 0x08]))
        .big_endian();

    let samples = decode(&wem)?;

    assert_eq!(
        samples.get(..5),
        Some(&[100, 113, 83, 79, 82][..]),
        "the same samples as little-endian"
    );
    Ok(())
}

#[test]
fn ima_step_indexes_out_of_range_are_clamped_and_samples_saturate() -> TestResult {
    let mut data = ima_block([0, 0], 100, &[0xF7]);
    data.extend(ima_block(100i16.to_le_bytes(), 0xFF, &[0x07]));
    let wem = TestWem::encoded(IMA, 1, 48_000, 0)
        .block_align(0x24)
        .data(data);

    let samples = decode(&wem)?;

    assert_eq!(
        samples.get(..3),
        Some(&[0, 32_767, -28_671][..]),
        "index 100 acts as 88, the sum saturates at i16::MAX"
    );
    assert_eq!(
        samples.get(64..66),
        Some(&[100, 113][..]),
        "the signed index 0xFF, -1, acts as 0"
    );
    Ok(())
}
/// A 36-byte PTADPCM frame: two history samples, the step index, then nibbles.
fn pt_frame(older: i16, newer: i16, index: u8, nibbles: &[u8]) -> Vec<u8> {
    let mut frame = older.to_le_bytes().to_vec();
    frame.extend_from_slice(&newer.to_le_bytes());
    frame.push(index);
    frame.extend_from_slice(nibbles);
    frame.resize(0x24, 0);
    frame
}

#[test]
fn a_mono_ptadpcm_frame_decodes_to_both_history_samples_and_every_nibble() -> TestResult {
    let wem = TestWem::encoded(PTADPCM, 1, 48_000, 0)
        .block_align(0x24)
        .data(pt_frame(10, 20, 0, &[0xF0, 0x87]));

    let samples = decode(&wem)?;

    assert_eq!(
        samples.len(),
        64,
        "2 history samples and 62 nibbles per frame"
    );
    assert_eq!(
        samples.get(..6),
        Some(&[10, 20, 16, 68, 112, 160][..]),
        "nibbles 0, F, 7, 8 low first: step + 2 × previous − the one before"
    );
    Ok(())
}

#[test]
fn stereo_ptadpcm_frames_decode_interleaved_and_headers_stay_little_endian() -> TestResult {
    let mut data = pt_frame(10, 20, 0, &[0xF0, 0x87]);
    data.extend(pt_frame(-5, -6, 0, &[0x0F]));
    let wem = TestWem::encoded(PTADPCM, 2, 48_000, 0)
        .block_align(0x48)
        .data(data);

    for wem in [wem.clone(), wem.big_endian()] {
        let samples = decode(&wem)?;

        assert_eq!(samples.len(), 2 * 64, "64 frames");
        assert_eq!(
            samples.get(..6),
            Some(&[10, -5, 20, -6, 16, 7][..]),
            "left and right alternate"
        );
    }
    Ok(())
}

#[test]
fn a_ptadpcm_step_index_above_12_has_no_steps() -> TestResult {
    let wem = TestWem::encoded(PTADPCM, 1, 48_000, 0)
        .block_align(0x24)
        .data(pt_frame(10, 20, 200, &[0x77]));

    let samples = decode(&wem)?;

    assert_eq!(
        samples.get(..4),
        Some(&[10, 20, 30, 40][..]),
        "index 12 adds no step and leads to index 0, whose nibble 7 adds none either"
    );
    Ok(())
}
