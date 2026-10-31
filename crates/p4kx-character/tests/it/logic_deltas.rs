// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The joint delta records a `RigLogic` rig carries for its own head: the reader and the face rig
//! made of them.

use std::error::Error;

use p4kx_character::{FaceRig, RigError, RigLogic, RigLogicError};
use p4kx_testkit::{TestLogicRecord, TestRigLogic};

type TestResult = Result<(), Box<dyn Error>>;

const SCALES_AT: usize = 0x48;
const COUNTS_AT: usize = 0x58;

fn with_main(record: TestLogicRecord) -> TestRigLogic {
    let mut logic = TestRigLogic::with_deltas();
    if let Some(deltas) = logic.deltas.as_mut() {
        deltas.main = vec![record];
    }
    logic
}

fn assert_refused(bytes: &[u8], reason: &'static str) {
    assert_eq!(
        RigLogic::read(bytes).err(),
        Some(RigLogicError::Malformed(reason)),
        "{reason}"
    );
}

#[test]
fn a_rig_with_delta_records_moves_its_own_joints() -> TestResult {
    let logic = RigLogic::read(&TestRigLogic::with_deltas().build_rig())?;
    assert!(logic.has_joint_deltas());
    let face = FaceRig::from_logic(&logic)?;
    assert_eq!(face.joints(), 2);

    let moved = face.pose(&[0.0, 1.0, 0.0]);
    assert!((moved.translations[1][0] - 0.01).abs() < 1e-6);
    assert_eq!(moved.translations[0], [0.0; 3]);
    assert_eq!(moved.rotations[1], [0.0, 0.0, 0.0, 1.0]);

    let turned = face.pose(&[0.0, 0.0, 1.0]);
    let half = 0.5_f32 / 2.0;
    assert!((turned.rotations[1][1] - half.sin()).abs() < 1e-5);
    assert!((turned.rotations[1][3] - half.cos()).abs() < 1e-5);
    assert_eq!(turned.translations[1], [0.0; 3]);
    Ok(())
}

#[test]
fn a_corrective_moves_the_joints_by_the_product_of_its_controls() -> TestResult {
    let record = TestLogicRecord {
        output: 5,
        entries: vec![(3, 20_000)],
    };
    let logic = RigLogic::read(&with_main(record).build_rig())?;
    let face = FaceRig::from_logic(&logic)?;

    let pose = face.pose(&[0.0, 0.5, 0.5]);

    assert!((pose.translations[1][2] - 0.005).abs() < 1e-6);
    Ok(())
}

#[test]
fn the_chunk_and_the_file_form_read_alike() -> TestResult {
    let logic = TestRigLogic::with_deltas();
    assert_eq!(
        RigLogic::read(&logic.build())?,
        RigLogic::read(&logic.build_rig())?
    );
    Ok(())
}

#[test]
fn a_rig_without_records_has_no_joint_deltas() -> TestResult {
    let logic = RigLogic::read(&TestRigLogic::small().build_rig())?;
    assert!(!logic.has_joint_deltas());
    assert_eq!(
        FaceRig::from_logic(&logic).err(),
        Some(RigError::LogicWithoutDeltas)
    );
    Ok(())
}

#[test]
fn a_record_that_moves_a_joint_the_rig_lacks_is_refused() {
    let record = TestLogicRecord {
        output: 6,
        entries: vec![(1, 1)],
    };
    assert_refused(
        &with_main(record).build_rig(),
        "a delta record moves a joint the rig lacks",
    );
}

#[test]
fn a_record_that_names_a_blendshape_the_rig_lacks_is_refused() {
    let record = TestLogicRecord {
        output: 3,
        entries: vec![(4, 1)],
    };
    assert_refused(
        &with_main(record).build_rig(),
        "a delta record names a blendshape the rig lacks",
    );
}

#[test]
fn more_records_than_the_section_holds_are_refused() {
    let mut bytes = TestRigLogic::with_deltas().build_rig();
    bytes[COUNTS_AT..COUNTS_AT + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_refused(&bytes, "a delta record is cut short by its section");
}

#[test]
fn a_scale_that_is_not_a_positive_number_is_refused() {
    for scale in [0.0_f64, -1.0, f64::NAN, f64::INFINITY] {
        let mut bytes = TestRigLogic::with_deltas().build_rig();
        bytes[SCALES_AT..SCALES_AT + 8].copy_from_slice(&scale.to_le_bytes());
        assert_refused(&bytes, "a delta scale is not a positive number");
    }
}

#[test]
fn a_header_that_counts_records_in_a_missing_section_is_refused() {
    let mut bytes = TestRigLogic::with_deltas().build_rig();
    bytes.truncate(0xa8 + 4);
    assert!(RigLogic::read(&bytes).is_err());
}
