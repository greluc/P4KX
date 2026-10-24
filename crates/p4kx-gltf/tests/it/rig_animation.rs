// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Animations of a rigged GLB: keys per bone, and poses and morph weights at sample times.

use std::error::Error;

use p4kx_gltf::{
    GlbOptions, MorphTarget, RigAnimation, RigPiece, SampledAnimation, SampledBone, SampledWeights,
    write_rigged_glb_animated,
};
use p4kx_model::MaterialStatus;
use p4kx_testkit::gltf::animation::Property;
use p4kx_testkit::{TestModel, check_glb};

use crate::rig::{body, piece_model, skeleton_model};

type TestResult = Result<(), Box<dyn Error>>;

fn targets() -> [MorphTarget; 2] {
    [
        MorphTarget {
            name: "smile".to_owned(),
            vertices: vec![1, 2],
            deltas: vec![[0.0, 0.0, 0.01]; 2],
        },
        MorphTarget {
            name: "blink".to_owned(),
            vertices: vec![0],
            deltas: vec![[0.0, 0.01, 0.0]],
        },
    ]
}

fn sampled() -> SampledAnimation {
    SampledAnimation::new(
        "talk".to_owned(),
        vec![0.0, 0.5, 1.0],
        vec![SampledBone::new(
            1,
            vec![[0.0, 0.0, 1.0], [0.0, 0.1, 1.0], [0.0, 0.2, 1.0]],
            vec![[0.0, 0.0, 0.0, 1.0]; 3],
        )],
        vec![SampledWeights::new(0, vec![0.0, 0.0, 0.5, 0.25, 1.0, 0.5])],
        vec![7],
    )
}

#[test]
fn a_sampled_animation_moves_bones_and_sets_morph_weights() -> TestResult {
    let skeleton = skeleton_model(&body())?;
    let piece = piece_model(&TestModel::two_bone_skin())?;
    let materials = MaterialStatus::Unnamed;
    let morphs = targets();
    let pieces = [RigPiece::new("head", &piece, &materials).with_morphs(&morphs)];
    let animation = sampled();
    let mut bytes = Vec::new();

    let summary = write_rigged_glb_animated(
        &skeleton.bones,
        &pieces,
        &[RigAnimation::Sampled(&animation)],
        &GlbOptions::default(),
        &mut bytes,
    )?;

    let glb = check_glb(&bytes)?;
    assert_eq!(summary.animations.len(), 1);
    assert_eq!(summary.animations[0].channels, 3);
    assert_eq!(summary.animations[0].skipped_tracks, [7]);
    let written = glb.document.animations().next().ok_or("an animation")?;
    assert_eq!(written.name(), Some("talk"));
    let weights = written
        .channels()
        .find(|channel| channel.target().property() == Property::MorphTargetWeights)
        .ok_or("a weights channel")?;
    assert_eq!(
        glb.floats(&weights.sampler().output()),
        [0.0, 0.0, 0.5, 0.25, 1.0, 0.5]
    );
    let translation = written
        .channels()
        .find(|channel| channel.target().property() == Property::Translation)
        .ok_or("a translation channel")?;
    assert_eq!(
        glb.floats(&translation.sampler().output())[3..6],
        [0.0, 1.0, -0.1]
    );
    Ok(())
}

#[test]
fn weights_that_do_not_fit_the_targets_are_left_out() -> TestResult {
    let skeleton = skeleton_model(&body())?;
    let piece = piece_model(&TestModel::two_bone_skin())?;
    let materials = MaterialStatus::Unnamed;
    let morphs = targets();
    let pieces = [RigPiece::new("head", &piece, &materials).with_morphs(&morphs)];
    let mut animation = sampled();
    animation.weights = vec![SampledWeights::new(0, vec![0.0, 1.0, 0.5])];
    let mut bytes = Vec::new();

    let summary = write_rigged_glb_animated(
        &skeleton.bones,
        &pieces,
        &[RigAnimation::Sampled(&animation)],
        &GlbOptions::default(),
        &mut bytes,
    )?;

    check_glb(&bytes)?;
    assert_eq!(summary.animations[0].channels, 2);
    assert_eq!(summary.animations[0].skipped_weights, 1);
    Ok(())
}
