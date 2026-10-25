// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The expression blendshapes of a Character's head: the stock heads' blendshapes mixed by the
//! DNA and the region masks, and the names of the blendshapes.

use std::error::Error;

use p4kx_character::{DnaRig, RigLogic, RigLogicError};
use p4kx_testkit::{TestCell, TestDna, TestRigLogic};

use crate::rig::dna;

type TestResult = Result<(), Box<dyn Error>>;

fn cell(variant: usize, entries: &[(u8, [u8; 3])]) -> TestCell {
    TestCell {
        variant,
        blendshape: 0,
        start_vertex: 0,
        scale: [0.01; 3],
        bias: [0.0; 3],
        entries: entries.to_vec(),
    }
}

fn rig() -> TestDna {
    let mut rig = TestDna::small();
    rig.blendshapes = 2;
    rig.cells = vec![
        TestCell {
            start_vertex: 0,
            entries: vec![(0, [10, 0, 0]), (2, [20, 0, 0])],
            ..cell(0, &[])
        },
        cell(1, &[(0, [0, 40, 0]), (1, [0, 40, 0])]),
        TestCell {
            start_vertex: 1,
            entries: vec![(0, [0, 0, 80]), (1, [0, 0, 80]), (1, [0, 0, 80])],
            ..cell(2, &[])
        },
    ];
    rig
}

fn close(actual: &[[f32; 3]], expected: &[[f32; 3]]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{actual:?} against {expected:?}"
    );
    for (actual, expected) in actual.iter().zip(expected) {
        for (a, e) in actual.iter().zip(expected) {
            assert!((a - e).abs() < 1e-5, "{actual:?} against {expected:?}");
        }
    }
}

#[test]
fn the_heads_blendshapes_are_mixed_by_the_region_masks_and_the_dna() -> TestResult {
    let bytes = rig().build();
    let rig = DnaRig::read(&bytes)?;
    let matrix = rig.matrix(&dna(&[&[(65535, 1)], &[], &[(65535, 2)]]))?;

    let blend = rig.expression_blend(&matrix)?;
    let target = blend.target(0);

    assert_eq!((blend.blendshapes(), blend.vertices()), (2, 4));
    assert_eq!(target.vertices, [0, 1, 2, 3]);
    close(
        &target.deltas,
        &[
            [0.0, 0.4, 0.0],
            [0.0, 0.2, 0.4],
            [0.0, 0.0, 0.8],
            [0.0, 0.0, 0.8],
        ],
    );
    Ok(())
}

#[test]
fn what_the_heads_leave_of_a_vertex_belongs_to_the_base_head() -> TestResult {
    let bytes = rig().build();
    let rig = DnaRig::read(&bytes)?;
    let matrix = rig.matrix(&dna(&[&[(65535, 1)], &[], &[]]))?;

    let target = rig.expression_blend(&matrix)?.target(0);

    assert_eq!(target.vertices, [0, 1, 2]);
    close(
        &target.deltas,
        &[[0.0, 0.4, 0.0], [0.0, 0.2, 0.0], [0.2, 0.0, 0.0]],
    );
    Ok(())
}

#[test]
fn a_dna_without_blends_gives_the_base_heads_blendshape() -> TestResult {
    let bytes = rig().build();
    let rig = DnaRig::read(&bytes)?;
    let matrix = rig.matrix(&dna(&[&[], &[], &[]]))?;

    let blend = rig.expression_blend(&matrix)?;

    close(&blend.target(0).deltas, &[[0.1, 0.0, 0.0], [0.2, 0.0, 0.0]]);
    assert_eq!(blend.target(0).vertices, [0, 2]);
    Ok(())
}

#[test]
fn an_empty_blendshape_and_one_outside_the_rig_give_an_empty_target() -> TestResult {
    let bytes = rig().build();
    let rig = DnaRig::read(&bytes)?;
    let matrix = rig.matrix(&dna(&[&[(65535, 1)], &[], &[]]))?;

    let blend = rig.expression_blend(&matrix)?;

    assert_eq!(blend.target(1), blend.target(99));
    assert_eq!(blend.target(1).vertices.len(), 0);
    Ok(())
}

#[test]
fn a_matrix_of_another_rig_is_refused() -> TestResult {
    let bytes = rig().build();
    let rig = DnaRig::read(&bytes)?;
    let other = p4kx_character::DnaMatrix::from_dna(&dna(&[&[]]), 2, 1)?;

    assert!(rig.expression_blend(&other).is_err());
    Ok(())
}

#[test]
fn the_names_of_the_controls_and_the_correctives_are_read() -> TestResult {
    let logic = RigLogic::read(&TestRigLogic::small().build())?;
    assert_eq!(RigLogic::read(&TestRigLogic::small().build_rig())?, logic);

    assert_eq!((logic.controls(), logic.blendshapes()), (3, 4));
    assert_eq!(logic.name(0).as_deref(), Some("CTL_PSD"));
    assert_eq!(logic.name(2).as_deref(), Some("CTL_b"));
    assert_eq!(logic.name(3).as_deref(), Some("CTL_a*CTL_b"));
    assert_eq!(logic.name(4), None);
    Ok(())
}

#[test]
fn a_control_is_found_by_its_stored_hash_and_a_table_that_disagrees_with_the_names_is_refused()
-> TestResult {
    let good = TestRigLogic::small().build_rig();
    let logic = RigLogic::read(&good)?;
    assert_eq!(logic.control_by_hash(crc32fast::hash(b"CTL_a")), Some(1));
    assert_eq!(logic.control_by_hash(crc32fast::hash(b"CTL_c")), None);

    let at = usize::try_from(u64::from_le_bytes(good[0x70..0x78].try_into()?))?;
    let mut wrong = good.clone();
    wrong[at + 4] ^= 1;
    assert_eq!(
        RigLogic::read(&wrong).err(),
        Some(RigLogicError::Malformed(
            "a stored control hash is not the CRC-32 of the control's name"
        ))
    );
    let mut short = good;
    let names = u64::from_le_bytes(short[0x68..0x70].try_into()?);
    short[0x70..0x78].copy_from_slice(&(names - 2).to_le_bytes());
    assert!(RigLogic::read(&short).is_err());
    Ok(())
}

#[test]
fn a_chunk_that_is_not_a_rig_logic_chunk_is_refused() {
    let good = TestRigLogic::small().build_rig();

    assert_eq!(
        RigLogic::read(&good[..20]).err(),
        Some(RigLogicError::Truncated {
            need: 168,
            have: 20
        })
    );
    let mut magic = good.clone();
    magic[0] = b'X';
    assert_eq!(RigLogic::read(&magic).err(), Some(RigLogicError::BadMagic));
    let mut header = good.clone();
    header[0x3e] = 7;
    assert!(RigLogic::read(&header).is_err());
    let mut count = good.clone();
    count[0x40] = 9;
    assert!(RigLogic::read(&count).is_err());
    let mut corrective = good.clone();
    corrective[0x42] = 200;
    assert!(RigLogic::read(&corrective).is_err());
    let mut start = good;
    start[0x68..0x70].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(RigLogic::read(&start).is_err());
}
