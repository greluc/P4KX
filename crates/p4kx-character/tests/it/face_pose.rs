// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The rig logic of a head: controls to blendshape activations and joint motion.

use std::error::Error;

use p4kx_character::{DnaRig, FaceRig, RigError, RigLogic};
use p4kx_testkit::{TestDna, TestRigLogic};

use crate::rig::sample_dna;

type TestResult = Result<(), Box<dyn Error>>;

fn rig() -> TestDna {
    let mut rig = TestDna::small();
    rig.blendshapes = 4;
    rig.deltas = vec![
        [Vec::new(), Vec::new()],
        [vec![(3, vec![(1 << 9) | 1, (2 << 9) | 3])], Vec::new()],
        [Vec::new(), vec![(5, vec![(1 << 9) | 2])]],
    ];
    rig.delta_scales = vec![(1.0, 1.0), (4_194_303.0, 1.0), (1.0, 4_194_303.0)];
    rig
}

fn logic() -> Result<RigLogic, Box<dyn Error>> {
    Ok(RigLogic::read(&TestRigLogic::small().build())?)
}

#[test]
fn controls_become_activations_and_correctives_are_products() -> TestResult {
    let logic = logic()?;
    let activations = logic.activations(&[0.0, 0.5, 0.5]);
    assert_eq!(activations, [1.0, 0.5, 0.5, 0.25]);
    let short = logic.activations(&[9.0]);
    assert_eq!(short, [1.0, 0.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn controls_and_joints_are_found_by_the_crc_of_their_names() -> TestResult {
    let logic = logic()?;
    assert_eq!(logic.control_by_hash(crc32fast::hash(b"CTL_b")), Some(2));
    assert_eq!(logic.control_by_hash(1), None);
    assert_eq!(
        logic.joint_hashes(),
        [
            crc32fast::hash(b"FACIAL_root"),
            crc32fast::hash(b"FACIAL_jaw")
        ]
    );
    Ok(())
}

#[test]
fn the_pose_sums_the_records_of_the_blended_heads_by_their_joint_weights() -> TestResult {
    let bytes = rig().build();
    let dna_rig = DnaRig::read(&bytes)?;
    let matrix = dna_rig.matrix(&sample_dna())?;
    let face = FaceRig::new(&dna_rig, &matrix, &logic()?)?;
    assert_eq!(face.joints(), 2);

    let pose = face.pose(&[0.0, 0.5, 0.5]);

    assert_eq!(pose.translations.len(), 2);
    assert!((pose.translations[1][0] - 0.375).abs() < 1e-5);
    assert_eq!(pose.translations[0], [0.0; 3]);
    let angle = 0.3125_f32 / 2.0;
    assert!((pose.rotations[1][2] - angle.sin()).abs() < 1e-5);
    assert!((pose.rotations[1][3] - angle.cos()).abs() < 1e-5);
    assert_eq!(pose.rotations[0], [0.0, 0.0, 0.0, 1.0]);
    Ok(())
}

#[test]
fn the_rest_pose_has_no_motion() -> TestResult {
    let bytes = rig().build();
    let dna_rig = DnaRig::read(&bytes)?;
    let matrix = dna_rig.matrix(&sample_dna())?;
    let face = FaceRig::new(&dna_rig, &matrix, &logic()?)?;

    let pose = face.pose(&[]);

    assert!(
        pose.translations
            .iter()
            .flatten()
            .all(|value| *value == 0.0_f32)
    );
    Ok(())
}

#[test]
fn a_record_that_names_a_missing_blendshape_is_malformed() -> TestResult {
    let mut broken = rig();
    broken.deltas[1][0] = vec![(3, vec![(1 << 9) | 9])];
    let bytes = broken.build();
    let dna_rig = DnaRig::read(&bytes)?;
    let matrix = dna_rig.matrix(&sample_dna())?;

    let error = FaceRig::new(&dna_rig, &matrix, &logic()?).err();

    assert!(
        matches!(error, Some(RigError::Malformed { .. })),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_record_that_moves_a_missing_joint_is_malformed() -> TestResult {
    let mut broken = rig();
    broken.deltas[1][0] = vec![(60, vec![(1 << 9) | 1])];
    let bytes = broken.build();
    let dna_rig = DnaRig::read(&bytes)?;
    let matrix = dna_rig.matrix(&sample_dna())?;

    let error = FaceRig::new(&dna_rig, &matrix, &logic()?).err();

    assert!(
        matches!(error, Some(RigError::Malformed { .. })),
        "{error:?}"
    );
    Ok(())
}
