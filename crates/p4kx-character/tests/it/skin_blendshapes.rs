// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The `BShapesGpu` chunk of a head's skin geometry: the reader and its refusals.

use std::error::Error;

use p4kx_character::{SkinBlendshapes, SkinBlendshapesError};
use p4kx_testkit::{TestSkinBlendshapes, TestSkinEntry};

type TestResult = Result<(), Box<dyn Error>>;

const NONE: [u32; 0] = [];

fn close(actual: [f32; 3], expected: [f32; 3]) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(actual, expected)| (actual - expected).abs() < 1e-6)
}

fn assert_refused(bytes: &[u8], reason: &'static str) {
    assert_eq!(
        SkinBlendshapes::read(bytes).err(),
        Some(SkinBlendshapesError::Malformed(reason)),
        "{reason}"
    );
}

#[test]
fn a_blendshape_is_the_codes_of_the_vertices_it_moves_by_its_scale_and_bias() -> TestResult {
    let bytes = TestSkinBlendshapes::small().build();
    let shapes = SkinBlendshapes::read(&bytes)?;
    assert_eq!((shapes.blendshapes(), shapes.vertices()), (3, 4));

    let first = shapes.target(1);
    assert_eq!(first.vertices, [1, 3]);
    assert!(close(first.deltas[0], [0.05, 0.0, 0.0]));
    assert!(close(first.deltas[1], [0.0, 0.0, 0.1]));

    let second = shapes.target(2);
    assert_eq!(second.vertices, [1]);
    assert!(close(second.deltas[0], [0.0, 0.05, 0.0]));
    Ok(())
}

#[test]
fn a_blendshape_no_vertex_has_or_outside_the_head_is_empty() -> TestResult {
    let bytes = TestSkinBlendshapes::small().build();
    let shapes = SkinBlendshapes::read(&bytes)?;
    assert_eq!(shapes.target(0).vertices, NONE);
    assert_eq!(shapes.target(3).vertices, NONE);
    assert_eq!(shapes.target(usize::MAX).vertices, NONE);
    Ok(())
}

#[test]
fn moves_under_a_tenth_of_a_micrometre_are_dropped() -> TestResult {
    let mut chunk = TestSkinBlendshapes::small();
    chunk.entries = vec![TestSkinEntry {
        vertex: 2,
        blendshape: 1,
        codes: [100, 100, 100],
    }];
    let bytes = chunk.build();
    assert_eq!(SkinBlendshapes::read(&bytes)?.target(1).vertices, NONE);
    Ok(())
}

#[test]
fn a_chunk_shorter_than_its_header_is_refused() {
    assert_eq!(
        SkinBlendshapes::read(&[0; 31]).err(),
        Some(SkinBlendshapesError::Truncated { need: 32, have: 31 })
    );
}

#[test]
fn a_table_outside_the_chunk_is_refused() {
    let bytes = TestSkinBlendshapes::small().build();
    let cut = bytes.len() - 1;
    assert_refused(&bytes[..cut], "a table lies outside the chunk");
}

#[test]
fn a_count_that_overflows_the_tables_is_refused() {
    let mut bytes = TestSkinBlendshapes::small().build();
    bytes[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_refused(&bytes, "a table lies outside the chunk");
}

#[test]
fn the_number_of_blendshapes_must_fit_a_bitmap() {
    for count in [0_u32, 513] {
        let mut bytes = TestSkinBlendshapes::small().build();
        bytes[0..4].copy_from_slice(&count.to_le_bytes());
        assert_refused(&bytes, "the number of blendshapes is not 1 to 512");
    }
}

#[test]
fn a_bitmap_that_does_not_count_the_entries_of_its_vertex_is_refused() {
    let mut bytes = TestSkinBlendshapes::small().build();
    let bitmaps_at = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]) as usize;
    bytes[bitmaps_at] |= 1;
    assert_refused(&bytes, "the bitmap of a vertex does not count its entries");
}

#[test]
fn a_bit_beyond_the_blendshapes_of_the_head_is_refused() {
    let mut chunk = TestSkinBlendshapes::small();
    chunk.entries.push(TestSkinEntry {
        vertex: 0,
        blendshape: 3,
        codes: [0; 3],
    });
    assert_refused(
        &chunk.build(),
        "a vertex is moved by a blendshape the head lacks",
    );
}
