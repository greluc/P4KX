// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Animations of a Character: keys that move the bones of the body, and facial animations
//! evaluated through the rig logic into morph weights.

use std::error::Error;

use p4kx_assembly::{
    ArchiveModels, Armour, AssemblyOptions, BodyVariant, Expressions, IssueKind, Piece,
};
use p4kx_chunks::ChunkFile;
use p4kx_extraction::Archive;
use p4kx_gltf::GlbOptions;
use p4kx_model::Animation;
use p4kx_testkit::{
    HEAD_VERTICES, MALE_RIG_HASH, TestAnimation, TestSkeleton, TestTrack, check_glb, head_entries,
    head_rig,
};

use crate::assembly_support::{Fixture, TestResult, fixture};
use crate::head_build::{dna_of_head_two_in_region_three, head_path};

fn animation(source: &TestAnimation) -> Result<Animation, Box<dyn Error>> {
    let bytes = source.caf();
    Ok(Animation::read_caf(
        source.name(),
        &ChunkFile::parse(bytes.as_slice())?,
    )?)
}

fn face_animation() -> TestAnimation {
    TestAnimation::new("blink.caf", 30).track(
        TestTrack::new(crc32fast::hash(b"CTL_a")).positions(&[
            (0, [0.0, 0.0, 0.0]),
            (10, [1.0, 0.0, 0.0]),
            (20, [0.0, 0.0, 0.0]),
        ]),
    )
}

fn body_animation() -> TestAnimation {
    TestAnimation::new("nod.caf", 30).track(
        TestTrack::new(TestSkeleton::controller_id(2))
            .rotations(&[(0, [0.0, 0.0, 0.0, 1.0]), (10, [0.0, 0.0, 0.0, 1.0])]),
    )
}

fn build() -> Result<(Armour, Archive, Fixture), Box<dyn Error>> {
    let fixture = fixture(head_entries(Some(
        head_rig(HEAD_VERTICES, MALE_RIG_HASH).build(),
    )))?;
    let archive = Archive::open(&fixture.path)?;
    let piece = Piece::file("head", head_path(), None)
        .shaped(dna_of_head_two_in_region_three())
        .with_expressions(Expressions::All);
    let armour = Armour::build(
        BodyVariant::Male,
        &[piece],
        &ArchiveModels::new(&archive),
        &AssemblyOptions::default(),
    )?;
    Ok((armour, archive, fixture))
}

#[test]
fn a_facial_animation_sets_the_weights_of_the_morph_targets_and_a_body_animation_moves_bones()
-> TestResult {
    let (armour, archive, _dir) = build()?;
    let models = ArchiveModels::new(&archive);
    let list = [
        animation(&face_animation())?,
        animation(&body_animation())?,
    ];

    let animations = armour.animate(&list, &models);

    assert!(animations.issues().is_empty(), "{:?}", animations.issues());
    let [face] = animations.sampled() else {
        return Err("one facial animation".into());
    };
    assert_eq!(face.times, [0.0, 10.0 / 30.0, 20.0 / 30.0]);
    let weights = &face.weights.first().ok_or("weights")?.values;
    assert_eq!(weights, &[0.0, 0.0, 1.0, 0.0, 0.0, 0.0]);

    let mut bytes = Vec::new();
    let summary = armour.write_glb_animated(&animations, &GlbOptions::default(), &mut bytes)?;
    let glb = check_glb(&bytes)?;
    assert_eq!(glb.document.animations().count(), 2);
    assert_eq!(summary.animations.len(), 2);
    Ok(())
}

#[test]
fn an_animation_that_fits_nothing_is_listed_and_not_written() -> TestResult {
    let (armour, archive, _dir) = build()?;
    let models = ArchiveModels::new(&archive);
    let stranger = TestAnimation::new("stranger.caf", 30)
        .track(TestTrack::new(12345).positions(&[(0, [0.0; 3]), (5, [1.0, 0.0, 0.0])]));

    let list = [animation(&stranger)?];
    let animations = armour.animate(&list, &models);

    assert!(animations.is_empty());
    let codes: Vec<&str> = animations
        .issues()
        .iter()
        .map(|issue| issue.kind().code())
        .collect();
    assert_eq!(codes, ["animation_not_used"]);
    assert!(matches!(
        animations.issues().first().map(|issue| issue.kind()),
        Some(IssueKind::AnimationNotUsed { .. })
    ));
    Ok(())
}

#[test]
fn the_facial_joints_of_the_skin_join_the_skeleton() -> TestResult {
    let (armour, _archive, _dir) = build()?;

    assert!(armour.skeleton().bones.len() >= 3);
    assert!(armour.issues().is_empty(), "{:?}", armour.issues());
    Ok(())
}
