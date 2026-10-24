// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::model::BoneTransform;

use super::{Pose, Sample, Skeleton};
use crate::fixtures::{skin_model, swing, unrelated};
use crate::rigid::IDENTITY_MATRIX;

fn skeleton() -> Skeleton {
    let model = skin_model();
    Skeleton::of(&model.bones).unwrap_or_else(|| panic!("a skeleton"))
}

fn near(a: &[f32], b: &[f32], tolerance: f32) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tolerance)
}

fn evaluated(skeleton: &Skeleton, clip: Option<Sample<'_>>) -> Pose {
    let mut pose = Pose::rest(skeleton.bone_count());
    skeleton.evaluate(clip, &mut Vec::new(), &mut pose);
    pose
}

#[test]
fn the_bind_pose_moves_no_bone() {
    let skeleton = skeleton();

    let pose = evaluated(&skeleton, None);

    assert_eq!(pose.matrices().len(), 2, "one matrix per bone");
    for matrix in pose.matrices() {
        assert!(
            near(matrix, &IDENTITY_MATRIX, 1.0e-5),
            "{matrix:?} is the identity"
        );
    }
}

#[test]
fn an_animation_at_its_bind_key_leaves_the_pose_as_bound() {
    let skeleton = skeleton();
    let animation = swing("swing", 30);
    let bindings = skeleton.bind(&animation);

    let pose = evaluated(
        &skeleton,
        Some(Sample {
            animation: &animation,
            bindings: &bindings,
            frame: 0.0,
        }),
    );

    for matrix in pose.matrices() {
        assert!(near(matrix, &IDENTITY_MATRIX, 1.0e-2), "{matrix:?}");
    }
}

#[test]
fn the_animated_bone_turns_and_its_parent_stays() {
    let skeleton = skeleton();
    let animation = swing("swing", 30);
    let bindings = skeleton.bind(&animation);

    let pose = evaluated(
        &skeleton,
        Some(Sample {
            animation: &animation,
            bindings: &bindings,
            frame: 30.0,
        }),
    );

    let [root, tip] = pose.matrices() else {
        panic!("two matrices");
    };
    assert!(near(root, &IDENTITY_MATRIX, 1.0e-5), "the root stays");
    assert!(!near(tip, &IDENTITY_MATRIX, 0.1), "the tip moved: {tip:?}");
    assert!((tip[15] - 1.0).abs() < 1.0e-6, "a rigid matrix");
}

#[test]
fn a_frame_between_two_keys_lies_between_their_poses() {
    let skeleton = skeleton();
    let animation = swing("swing", 30);
    let bindings = skeleton.bind(&animation);
    let at = |frame| {
        evaluated(
            &skeleton,
            Some(Sample {
                animation: &animation,
                bindings: &bindings,
                frame,
            }),
        )
        .matrices()
        .last()
        .copied()
        .unwrap_or_default()
    };

    let (start, middle, end) = (at(0.0), at(15.0), at(30.0));

    let distance =
        |a: &[f32; 16], b: &[f32; 16]| -> f32 { a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum() };
    assert!(distance(&start, &middle) > 0.05, "moved from the start");
    assert!(distance(&middle, &end) > 0.05, "short of the end");
}

#[test]
fn a_frame_after_the_last_key_holds_the_last_pose() {
    let skeleton = skeleton();
    let animation = swing("swing", 30);
    let bindings = skeleton.bind(&animation);
    let at = |frame| {
        evaluated(
            &skeleton,
            Some(Sample {
                animation: &animation,
                bindings: &bindings,
                frame,
            }),
        )
    };

    assert_eq!(at(30.0).matrices(), at(500.0).matrices(), "held");
}

#[test]
fn only_an_animation_with_a_track_for_a_bone_moves_the_rig() {
    let skeleton = skeleton();

    assert!(skeleton.moves(&swing("swing", 30)), "binds the tip");
    assert!(
        !skeleton.moves(&unrelated("other")),
        "another skeleton's bones"
    );
}

#[test]
fn the_static_bones_hold_the_bind_position_and_the_parent() {
    let skeleton = skeleton();

    let [root, tip] = skeleton.static_bones()[..] else {
        panic!("two bones");
    };

    assert_eq!(root[3], super::NO_PARENT, "a root has no parent");
    assert_eq!(tip[3], 0, "the tip hangs below the root");
    assert!(
        (f32::from_bits(tip[2]) - 2.0).abs() < 1.0e-5,
        "bound 2 m up"
    );
}

#[test]
fn a_skeleton_with_a_cycle_or_without_bones_has_no_rig() {
    assert!(Skeleton::of(&[]).is_none(), "no bones");
    let mut model = skin_model();
    model.bones[0].parent = Some(1);
    assert!(Skeleton::of(&model.bones).is_none(), "a cycle");
}

#[test]
fn the_bind_transform_of_a_root_is_what_the_file_stores() {
    let skeleton = skeleton();

    assert_eq!(
        skeleton.bones()[0].world,
        BoneTransform::IDENTITY,
        "root at the origin"
    );
    assert!(
        near(&skeleton.bones()[1].relative.rotation, &[0.5; 4], 1.0e-6),
        "tip turn"
    );
}
