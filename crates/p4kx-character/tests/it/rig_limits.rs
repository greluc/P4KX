// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Corrupt DNA rigs and DNA that does not fit them are errors, never panics, and the DNA matrix
//! blends as the game does.

use std::error::Error;

use p4kx_character::{DeltaSet, DnaMatrix, DnaRig, RigError};
use p4kx_testkit::TestDna;
use proptest::collection::vec;
use proptest::prelude::*;

use crate::rig::{dna, sample_dna};

type TestResult = Result<(), Box<dyn Error>>;

fn patched(mut bytes: Vec<u8>, at: usize, value: &[u8]) -> Vec<u8> {
    if let Some(target) = bytes.get_mut(at..at + value.len()) {
        target.copy_from_slice(value);
    }
    bytes
}

fn use_everything(bytes: &[u8]) {
    let Ok(rig) = DnaRig::read(bytes) else { return };
    let _ = rig.variant_names();
    let _ = rig.covered_regions();
    let _ = rig.vertex_count();
    let _ = rig.joint_count();
    let _ = rig.variant_joints();
    let _ = rig.joint_weights();
    let _ = rig.region_masks();
    if let Ok(matrix) = rig.matrix(&sample_dna()) {
        let _ = rig.neutral_joint_offsets(&matrix);
        if let Ok(morphs) = rig.neutral_morphs() {
            let _ = morphs.decode(&matrix);
        }
    }
    if let Ok(morphs) = rig.expression_morphs() {
        for variant in 0..morphs.variants() {
            for blendshape in 0..morphs.blendshapes() {
                if let Some(cell) = morphs.cell(variant, blendshape) {
                    cell.moves().for_each(drop);
                }
            }
        }
    }
    if let Ok(deltas) = rig.variant_deltas() {
        for variant in 0..rig.variants() {
            for set in [DeltaSet::Main, DeltaSet::Secondary] {
                if let Ok(records) = deltas.records(variant, set) {
                    for record in records.flatten() {
                        record.entries().for_each(drop);
                    }
                }
            }
        }
    }
}

#[test]
fn a_file_shorter_than_the_header_is_truncated() {
    let bytes = TestDna::small().build();

    let error = DnaRig::read(bytes.get(..0x13f).unwrap_or_default());

    assert!(
        matches!(
            error,
            Err(RigError::HeaderTruncated {
                need: 0x140,
                have: 0x13f
            })
        ),
        "{error:?}"
    );
}

#[test]
fn a_file_with_another_magic_is_refused() {
    let bytes = patched(TestDna::small().build(), 5, b"9");

    assert!(matches!(DnaRig::read(&bytes), Err(RigError::BadMagic)));
}

#[test]
fn a_rig_without_heads_is_refused() {
    let bytes = patched(TestDna::small().build(), 0x2a, &[0, 0]);

    assert!(matches!(DnaRig::read(&bytes), Err(RigError::ZeroVariants)));
}

#[test]
fn a_rig_with_more_cells_than_supported_is_refused() {
    let bytes = patched(TestDna::small().build(), 0x2a, &[0xff, 0xff]);
    let bytes = patched(bytes, 0x28, &[0xff]);

    assert!(matches!(
        DnaRig::read(&bytes),
        Err(RigError::TooLarge { .. })
    ));
}

#[test]
fn a_slot_outside_the_file_or_inside_the_header_is_refused() {
    let bytes = TestDna::small().build();
    let far = patched(bytes.clone(), 0x40 + 8 * 15, &u64::MAX.to_le_bytes());
    let near = patched(bytes, 0x40 + 8 * 15, &0x80_u64.to_le_bytes());

    for broken in [far, near] {
        assert!(
            matches!(
                DnaRig::read(&broken),
                Err(RigError::SlotOutOfRange { slot: 15, .. })
            ),
            "{:?}",
            DnaRig::read(&broken).err()
        );
    }
}

#[test]
fn a_cursor_that_does_not_count_the_set_controls_is_malformed() -> TestResult {
    let bytes = TestDna::small().build();
    let slot_at = 0x40 + 8 * 18;
    let slot = usize::try_from(u64::from_le_bytes(
        bytes.get(slot_at..slot_at + 8).ok_or("slot")?.try_into()?,
    ))?;
    let cursor_of_vertex_one = slot + 4 * 4 + 4;
    let broken = patched(bytes, cursor_of_vertex_one, &9_u32.to_le_bytes());

    let error = DnaRig::read(&broken)?.neutral_morphs().err();

    assert!(
        matches!(error, Some(RigError::Malformed { slot: 18, .. })),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_vertex_count_the_file_cannot_hold_is_too_short_not_an_allocation() -> TestResult {
    let bytes = TestDna::small().build();
    let slot_at = 0x40 + 8 * 22;
    let slot = u64::from_le_bytes(bytes.get(slot_at..slot_at + 8).ok_or("slot")?.try_into()?);
    let bytes = patched(bytes, slot_at, &(slot + 33).to_le_bytes());
    let bytes = patched(bytes, 0x30, &[0xff, 0xff, 0, 0]);

    let rig = DnaRig::read(&bytes)?;

    assert!(rig.region_masks().is_err());
    assert!(rig.neutral_morphs().is_err());
    Ok(())
}

#[test]
fn a_delta_record_cut_short_ends_the_records_with_an_error() -> TestResult {
    let bytes = TestDna::small().build();
    let cut = bytes.get(..bytes.len() - 6).ok_or("cut")?;
    let rig = DnaRig::read(cut)?;

    let records: Vec<_> = rig
        .variant_deltas()?
        .records(2, DeltaSet::Secondary)?
        .collect();

    assert_eq!(records.len(), 1);
    assert!(matches!(
        &records[0],
        Err(RigError::Malformed { slot: 31, .. })
    ));
    Ok(())
}

#[test]
fn a_blend_of_a_head_the_rig_lacks_is_refused() {
    let error = DnaMatrix::from_dna(&dna(&[&[(100, 3)]]), 3, 3);

    assert_eq!(
        error,
        Err(RigError::HeadOutOfRange {
            head_id: 3,
            variants: 3
        })
    );
}

#[test]
fn blends_in_a_face_part_the_rig_has_no_region_for_are_refused_unless_empty() {
    let parts: &[&[(u16, u16)]] = &[&[(1, 0)], &[], &[], &[(5, 1)]];
    assert!(matches!(
        DnaMatrix::from_dna(&dna(parts), 3, 3),
        Err(RigError::FacePartCount {
            parts: 4,
            regions: 3
        })
    ));
    let empty: &[&[(u16, u16)]] = &[&[(1, 0)], &[], &[], &[(0, 1)]];
    assert!(DnaMatrix::from_dna(&dna(empty), 3, 3).is_ok());
}

#[test]
fn the_weights_of_a_region_sum_to_one_and_a_region_without_blends_stays_zero() -> TestResult {
    let matrix = DnaMatrix::from_dna(&dna(&[&[(3000, 1), (1000, 2)], &[], &[(1, 2)]]), 3, 3)?;

    assert!((matrix.cell(1, 0) - 0.75).abs() < 1e-6);
    assert!((matrix.cell(2, 0) - 0.25).abs() < 1e-6);
    assert_eq!((0..3).map(|head| matrix.cell(head, 1)).sum::<f32>(), 0.0);
    assert!((matrix.cell(2, 2) - 1.0).abs() < 1e-6);
    assert_eq!(matrix.cell(9, 0), 0.0);
    assert_eq!(matrix.cell(0, 9), 0.0);
    Ok(())
}

#[test]
fn only_the_four_strongest_heads_of_a_region_are_kept() -> TestResult {
    let blends = [(10, 1), (20, 2), (30, 3), (40, 4), (50, 5)];
    let matrix = DnaMatrix::from_dna(&dna(&[&blends]), 6, 1)?;

    assert_eq!(matrix.cell(1, 0), 0.0, "the weakest is dropped");
    let kept: f32 = (0..6).map(|head| matrix.cell(head, 0)).sum();
    assert!((kept - 1.0).abs() < 1e-6, "{kept}");
    assert!((matrix.cell(5, 0) - 50.0 / 140.0).abs() < 1e-5);
    Ok(())
}

#[test]
fn blends_of_one_head_add_up_and_are_capped_at_one() -> TestResult {
    let matrix = DnaMatrix::from_dna(&dna(&[&[(65535, 1), (65535, 1), (65535, 2)]]), 3, 1)?;

    assert!((matrix.cell(1, 0) - 0.5).abs() < 1e-6);
    assert!((matrix.cell(2, 0) - 0.5).abs() < 1e-6);
    Ok(())
}

proptest! {
    #[test]
    fn patched_files_never_panic(patches in vec((0_usize..700, any::<u8>()), 0..24)) {
        let mut bytes = TestDna::small().build();
        for (at, value) in patches {
            if let Some(byte) = bytes.get_mut(at) {
                *byte = value;
            }
        }
        use_everything(&bytes);
    }

    #[test]
    fn arbitrary_bytes_never_panic(bytes in vec(any::<u8>(), 0..600)) {
        use_everything(&bytes);
    }
}
