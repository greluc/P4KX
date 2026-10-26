// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The DNA rig reader: header, names, joint offsets, neutral and expression morphs, joint and
//! region weights and delta records of a synthetic rig.

use std::error::Error;

use p4kx_character::{DeltaEntry, DeltaSet, Dna, DnaBlend, DnaRig, RigError};
use p4kx_testkit::TestDna;

type TestResult = Result<(), Box<dyn Error>>;

fn close(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 1e-3,
            "{actual:?} differs from {expected:?}"
        );
    }
}

pub(crate) fn dna(parts: &[&[(u16, u16)]]) -> Dna {
    Dna {
        gender_hash: 0xdd6c_67f6,
        label: p4kx_character::Label,
        variant_hash: 0,
        blends_per_part: 2,
        header_unknown: 0,
        max_head_id: 2,
        face_parts: parts
            .iter()
            .map(|blends| {
                blends
                    .iter()
                    .map(|&(value, head_id)| DnaBlend { value, head_id })
                    .collect()
            })
            .collect(),
    }
}

pub(crate) fn sample_dna() -> Dna {
    dna(&[&[(65535, 1)], &[(65535, 2)], &[(16384, 1), (49151, 2)]])
}

#[test]
fn the_header_names_the_rig_and_its_dimensions() -> TestResult {
    let bytes = TestDna::small().build();
    let rig = DnaRig::read(&bytes)?;

    let header = rig.header();
    assert_eq!(header.identity, 0x1122_3344_5566_7788);
    assert_eq!(header.rig_hash, 0xdd6c_67f6);
    assert_eq!((rig.variants(), rig.regions()), (3, 3));
    assert_eq!(header.vertices, 4);
    assert_eq!(rig.covered_regions()?, [0, 2]);
    assert_eq!(rig.vertex_count()?, 4);
    assert_eq!(rig.variant_names()?, ["base", "head_a", "head_b"]);
    Ok(())
}

#[test]
fn a_file_without_a_region_list_covers_every_region() -> TestResult {
    let mut test = TestDna::small();
    test.covered = Vec::new();
    test.masks = vec![vec![0.5; 4]; 3];
    test.controls.clear();
    let bytes = test.build();
    let rig = DnaRig::read(&bytes)?;

    assert_eq!(rig.covered_regions()?, [0, 1, 2]);
    assert_eq!(rig.vertex_count()?, 4);
    Ok(())
}

#[test]
fn the_vertex_count_survives_any_trailer_behind_the_region_masks() -> TestResult {
    for trailer in [0, 4, 8] {
        let mut test = TestDna::small();
        test.mask_trailer = trailer;
        let bytes = test.build();
        let rig = DnaRig::read(&bytes)?;

        assert_eq!(rig.vertex_count()?, 4, "trailer {trailer}");
        assert_eq!(rig.region_masks()?.weight(1, 3), Some(1.0), "{trailer}");
        assert_eq!(rig.neutral_morphs()?.vertices(), 4, "{trailer}");
    }
    Ok(())
}

#[test]
fn the_joint_offsets_of_each_head_are_read_per_joint() -> TestResult {
    let bytes = TestDna::small().build();
    let rig = DnaRig::read(&bytes)?;

    assert_eq!(rig.joint_count()?, 2);
    let joints = rig.variant_joints()?;
    assert_eq!((joints.variants(), joints.joints()), (3, 2));
    close(joints.translation(1, 0).ok_or("joint")?, [0.1, 0.0, 0.0]);
    close(joints.translation(2, 1).ok_or("joint")?, [1.0, 2.0, 3.5]);
    assert_eq!(joints.translation(3, 0), None);
    assert_eq!(joints.translation(0, 2), None);
    Ok(())
}

#[test]
fn the_joint_and_region_weights_are_read() -> TestResult {
    let bytes = TestDna::small().build();
    let rig = DnaRig::read(&bytes)?;

    let weights = rig.joint_weights()?;
    assert_eq!((weights.joints(), weights.regions()), (2, 3));
    assert_eq!(weights.weight(1, 2), Some(0.5));
    assert_eq!(weights.weight(0, 0), Some(0.0));
    assert_eq!(weights.weight(2, 0), None);

    let masks = rig.region_masks()?;
    assert_eq!(masks.vertices(), 4);
    assert_eq!(masks.weight(0, 1), Some(0.5));
    assert_eq!(masks.weight(1, 3), Some(1.0));
    assert_eq!(masks.weight(2, 0), None);
    assert_eq!(masks.weight(0, 4), None);
    Ok(())
}

#[test]
fn the_neutral_morphs_move_each_vertex_by_the_weights_of_the_dna() -> TestResult {
    let bytes = TestDna::small().build();
    let rig = DnaRig::read(&bytes)?;
    let morphs = rig.neutral_morphs()?;
    assert_eq!((morphs.vertices(), morphs.controls()), (4, 4));

    let moves = morphs.decode(&rig.matrix(&sample_dna())?)?;

    assert_eq!(moves.len(), 4);
    close(moves[0], [0.1, 0.2, 0.3]);
    close(moves[1], [1.6975, 2.0825, 2.4675]);
    close(moves[2], [0.0; 3]);
    close(moves[3], [75.1875, 37.6875, 0.1875]);
    Ok(())
}

#[test]
fn a_head_the_dna_does_not_use_moves_nothing() -> TestResult {
    let bytes = TestDna::small().build();
    let rig = DnaRig::read(&bytes)?;
    let morphs = rig.neutral_morphs()?;

    let moves = morphs.decode(&rig.matrix(&dna(&[&[(65535, 0)]]))?)?;

    assert!(
        moves.iter().flatten().all(|&value| value == 0.0),
        "{moves:?}"
    );
    Ok(())
}

#[test]
fn a_weight_per_control_moves_the_vertices_the_control_is_set_for() -> TestResult {
    let bytes = TestDna::small().build();
    let rig = DnaRig::read(&bytes)?;
    let morphs = rig.neutral_morphs()?;

    let moves = morphs.decode_weights(&[0.0, 0.0, 0.0, 2.0]);

    close(moves[1], [4.5, 5.5, 6.5]);
    close(moves[3], [200.5, 100.5, 0.5]);
    close(moves[0], [0.0; 3]);
    Ok(())
}

#[test]
fn the_joints_of_a_blended_head_sit_where_the_weights_put_them() -> TestResult {
    let bytes = TestDna::small().build();
    let rig = DnaRig::read(&bytes)?;

    let offsets = rig.neutral_joint_offsets(&rig.matrix(&sample_dna())?)?;

    assert_eq!(offsets.len(), 2);
    close(offsets[0], [0.0; 3]);
    close(offsets[1], [0.0, 0.1875, 0.3125]);
    Ok(())
}

#[test]
fn a_matrix_of_another_rig_is_refused() -> TestResult {
    let bytes = TestDna::small().build();
    let rig = DnaRig::read(&bytes)?;
    let other = p4kx_character::DnaMatrix::from_dna(&sample_dna(), 3, 4)?;

    assert!(matches!(
        rig.neutral_joint_offsets(&other),
        Err(RigError::MatrixShape { .. })
    ));
    assert!(matches!(
        rig.neutral_morphs()?.decode(&other),
        Err(RigError::MatrixShape { .. })
    ));
    Ok(())
}

#[test]
fn the_expression_morphs_give_the_vertices_a_blendshape_of_a_head_moves() -> TestResult {
    let bytes = TestDna::small().build();
    let rig = DnaRig::read(&bytes)?;
    let morphs = rig.expression_morphs()?;
    assert_eq!((morphs.variants(), morphs.blendshapes()), (3, 2));

    let cell = morphs.cell(1, 1).ok_or("the cell")?;
    assert_eq!(cell.len(), 2);
    let moves: Vec<(u32, [f32; 3])> = cell.moves().collect();
    assert_eq!(
        moves.iter().map(|(vertex, _)| *vertex).collect::<Vec<_>>(),
        [1, 3]
    );
    close(moves[0].1, [0.0, 1.0, 2.0]);
    close(moves[1].1, [1.0, 0.0, 1.5]);
    assert!(morphs.cell(0, 0).is_none(), "an empty cell");
    assert!(morphs.cell(3, 0).is_none(), "a head outside the table");
    assert!(
        morphs.cell(0, 2).is_none(),
        "a blendshape outside the table"
    );
    Ok(())
}

#[test]
fn the_delta_records_of_each_head_are_read_with_their_scales() -> TestResult {
    let bytes = TestDna::small().build();
    let rig = DnaRig::read(&bytes)?;
    let deltas = rig.variant_deltas()?;

    let header = deltas.header(1).ok_or("header")?;
    assert_eq!((header.main_records, header.secondary_records), (1, 0));
    assert!((header.step(DeltaSet::Main) - 1.0).abs() < 1e-12);
    assert!((header.step(DeltaSet::Secondary) - 2.0).abs() < 1e-12);

    let records: Vec<_> = deltas
        .records(1, DeltaSet::Main)?
        .collect::<Result<_, _>>()?;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].output, 3);
    assert_eq!(
        records[0].entries().collect::<Vec<_>>(),
        [
            DeltaEntry {
                blendshape: 7,
                coefficient: 2
            },
            DeltaEntry {
                blendshape: 1,
                coefficient: -3
            }
        ]
    );
    let secondary: Vec<_> = deltas
        .records(2, DeltaSet::Secondary)?
        .collect::<Result<_, _>>()?;
    assert_eq!(secondary.len(), 1);
    assert_eq!(secondary[0].output, 5);
    assert!(deltas.records(0, DeltaSet::Main)?.next().is_none());
    assert!(deltas.header(3).is_none());
    Ok(())
}
