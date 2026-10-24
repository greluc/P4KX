// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Animations written as GLB: channels on bone nodes and on the nodes of static models.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_gltf::{GlbOptions, GlbSummary, write_glb_animated};
use p4kx_model::{Animation, CompanionError, CompanionPath, Model};
use p4kx_testkit::gltf::animation::{Interpolation, Property};
use p4kx_testkit::{CheckedGlb, TestAnimation, TestModel, TestSkeleton, TestTrack, check_glb};

type TestResult = Result<(), Box<dyn Error>>;

const HALF: f32 = std::f32::consts::FRAC_1_SQRT_2;
const QUARTER_TURN_Z: [f32; 4] = [0.0, 0.0, HALF, HALF];
const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

fn model(source: &TestModel, name: &str) -> Result<Model, Box<dyn Error>> {
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Sibling(_) => Some(source.cgfm()),
            _ => None,
        })
    };
    Ok(Model::read(
        name,
        &ChunkFile::parse(source.cgf())?,
        &mut companions,
    )?)
}

fn animation(source: &TestAnimation) -> Result<Animation, Box<dyn Error>> {
    let file = ChunkFile::parse(source.caf())?;
    Ok(Animation::read_caf(source.name(), &file)?)
}

fn glb(
    model: &Model,
    animations: &[Animation],
) -> Result<(CheckedGlb, GlbSummary), Box<dyn Error>> {
    let mut bytes = Vec::new();
    let summary = write_glb_animated(model, animations, &GlbOptions::default(), &mut bytes)?;
    Ok((check_glb(&bytes)?, summary))
}

fn rig(source: &TestModel) -> Result<Model, Box<dyn Error>> {
    let mut companions =
        |_: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> { Ok(None) };
    Ok(Model::read(
        "rig.chr",
        &ChunkFile::parse(source.cgf())?,
        &mut companions,
    )?)
}

fn skin() -> Result<Model, Box<dyn Error>> {
    model(&TestModel::two_bone_skin(), "skin.skin")
}

fn tip() -> u32 {
    TestSkeleton::controller_id(1)
}

fn wave() -> TestAnimation {
    TestAnimation::new("wave.caf", 30).track(
        TestTrack::new(tip())
            .rotations(&[(0, IDENTITY), (30, QUARTER_TURN_Z)])
            .positions(&[(0, [0.0, 0.0, 2.0]), (30, [1.0, 0.0, 2.0])]),
    )
}

fn node_name(glb: &CheckedGlb, index: usize) -> Option<&str> {
    glb.document.nodes().nth(index).and_then(|node| node.name())
}

fn assert_close(actual: &[f32], expected: &[f32], what: &str) {
    assert_eq!(actual.len(), expected.len(), "{what}: length");
    for (a, e) in actual.iter().zip(expected) {
        assert!(
            (a - e).abs() < 1e-3,
            "{what}: {actual:?} is not {expected:?}"
        );
    }
}

#[test]
fn a_skinned_model_is_animated_through_its_bone_nodes() -> TestResult {
    let (glb, summary) = glb(&skin()?, &[animation(&wave())?])?;

    assert_eq!(glb.document.animations().len(), 1, "one animation");
    let animation = glb.document.animations().next().ok_or("an animation")?;
    assert_eq!(animation.name(), Some("wave"), "named after the file");
    let channels: Vec<_> = animation.channels().collect();
    assert_eq!(channels.len(), 2, "rotation and translation");
    for channel in &channels {
        let target = channel.target();
        assert_eq!(
            node_name(&glb, target.node().index()),
            Some("tip"),
            "the tip bone's node"
        );
        assert!(
            glb.document
                .skins()
                .flat_map(|skin| skin.joints().collect::<Vec<_>>())
                .any(|joint| joint.index() == target.node().index()),
            "the target is a joint of the skin"
        );
    }
    let [animation_summary] = summary.animations.as_slice() else {
        return Err("one animation summary".into());
    };
    assert_eq!(animation_summary.name, "wave", "summary name");
    assert_eq!(animation_summary.channels, 2, "summary channels");
    assert_eq!(animation_summary.keys, 4, "summary keys");
    assert!(
        (animation_summary.duration_seconds - 1.0).abs() < 1e-6,
        "30 frames at 30 fps"
    );
    assert!(
        animation_summary.skipped_tracks.is_empty(),
        "nothing skipped"
    );
    Ok(())
}

#[test]
fn times_are_seconds_and_values_are_in_y_up() -> TestResult {
    let (glb, _) = glb(&skin()?, &[animation(&wave())?])?;

    let animation = glb.document.animations().next().ok_or("an animation")?;
    for channel in animation.channels() {
        let sampler = channel.sampler();
        assert_close(
            &glb.floats(&sampler.input()),
            &[0.0, 1.0],
            "key times in seconds",
        );
        assert_eq!(sampler.interpolation(), Interpolation::Linear, "linear");
        let output = glb.floats(&sampler.output());
        match channel.target().property() {
            Property::Translation => {
                assert_close(&output, &[0.0, 2.0, 0.0, 1.0, 2.0, 0.0], "x, z, -y");
            }
            Property::Rotation => assert_close(
                &output,
                &[0.0, 0.0, 0.0, 1.0, 0.0, HALF, 0.0, HALF],
                "x, z, -y, w",
            ),
            other => return Err(format!("unexpected {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn the_frame_rate_of_each_animation_sets_its_times() -> TestResult {
    let slow = TestAnimation::new("slow.caf", 10)
        .track(TestTrack::new(tip()).rotations(&[(0, IDENTITY), (25, QUARTER_TURN_Z)]));

    let (glb, summary) = glb(&skin()?, &[animation(&slow)?])?;

    let channel = glb
        .document
        .animations()
        .next()
        .and_then(|animation| animation.channels().next())
        .ok_or("a channel")?;
    assert_close(
        &glb.floats(&channel.sampler().input()),
        &[0.0, 2.5],
        "25 frames at 10 fps",
    );
    assert!(
        summary
            .animations
            .first()
            .is_some_and(|animation| (animation.duration_seconds - 2.5).abs() < 1e-6),
        "duration"
    );
    Ok(())
}

#[test]
fn a_single_key_is_held() -> TestResult {
    let pose = TestAnimation::new("pose.caf", 30)
        .track(TestTrack::new(tip()).rotations(&[(0, QUARTER_TURN_Z)]));

    let (glb, _) = glb(&skin()?, &[animation(&pose)?])?;

    let channel = glb
        .document
        .animations()
        .next()
        .and_then(|animation| animation.channels().next())
        .ok_or("a channel")?;
    assert_eq!(
        channel.sampler().interpolation(),
        Interpolation::Step,
        "step"
    );
    Ok(())
}

#[test]
fn tracks_for_bones_the_skeleton_lacks_are_skipped_and_reported() -> TestResult {
    let mixed = TestAnimation::new("mixed.caf", 30)
        .track(TestTrack::new(tip()).rotations(&[(0, IDENTITY), (4, QUARTER_TURN_Z)]))
        .track(TestTrack::new(0xDEAD_BEEF).rotations(&[(0, IDENTITY)]))
        .track(TestTrack::new(0x1234).positions(&[(0, [0.0; 3])]));

    let (glb, summary) = glb(&skin()?, &[animation(&mixed)?])?;

    let animation = glb.document.animations().next().ok_or("an animation")?;
    assert_eq!(animation.channels().count(), 1, "only the tip's rotation");
    let [written] = summary.animations.as_slice() else {
        return Err("one animation summary".into());
    };
    assert_eq!(written.skipped_tracks, [0xDEAD_BEEF, 0x1234], "reported");
    Ok(())
}

#[test]
fn an_animation_without_a_matching_bone_is_not_written() -> TestResult {
    let stranger = TestAnimation::new("stranger.caf", 30)
        .track(TestTrack::new(0xFEED).rotations(&[(0, IDENTITY)]));

    let (glb, summary) = glb(&skin()?, &[animation(&stranger)?, animation(&wave())?])?;

    assert_eq!(
        glb.document.animations().len(),
        1,
        "only the animation that moves"
    );
    let channels: Vec<usize> = summary.animations.iter().map(|a| a.channels).collect();
    assert_eq!(channels, [0, 2], "the summary lists both");
    Ok(())
}

#[test]
fn several_animations_are_written_in_order() -> TestResult {
    let nod = TestAnimation::new("nod.caf", 30)
        .track(TestTrack::new(tip()).rotations(&[(0, IDENTITY), (6, QUARTER_TURN_Z)]));

    let (glb, summary) = glb(&skin()?, &[animation(&wave())?, animation(&nod)?])?;

    let names: Vec<Option<&str>> = glb.document.animations().map(|a| a.name()).collect();
    assert_eq!(names, [Some("wave"), Some("nod")], "in the order given");
    assert_eq!(summary.animations.len(), 2, "two summaries");
    Ok(())
}

#[test]
fn a_skeleton_file_is_animated_through_its_bone_nodes() -> TestResult {
    let skeleton = TestModel::cube()
        .helpers_only()
        .with_skeleton(&TestSkeleton::chain(3));
    let swing = TestAnimation::new("swing.caf", 30).track(
        TestTrack::new(TestSkeleton::controller_id(2))
            .rotations(&[(0, IDENTITY), (15, QUARTER_TURN_Z)]),
    );

    let (glb, summary) = glb(&rig(&skeleton)?, &[animation(&swing)?])?;

    let channel = glb
        .document
        .animations()
        .next()
        .and_then(|animation| animation.channels().next())
        .ok_or("a channel")?;
    assert_eq!(
        node_name(&glb, channel.target().node().index()),
        Some("bone_2"),
        "the third bone"
    );
    assert_eq!(summary.bones, 3, "bones written as nodes");
    Ok(())
}

#[test]
fn a_static_model_moves_the_nodes_its_helper_bones_drive() -> TestResult {
    let cube = TestModel::cube().with_compact_skeleton(&TestSkeleton::chain(2));
    let door = TestAnimation::new("door.caf", 30).track(
        TestTrack::new(TestSkeleton::controller_id(1))
            .rotations(&[(0, IDENTITY), (30, QUARTER_TURN_Z)]),
    );

    let (glb, summary) = glb(&model(&cube, "cube.cgf")?, &[animation(&door)?])?;

    assert_eq!(summary.bones, 0, "the helper bones are not written");
    let channel = glb
        .document
        .animations()
        .next()
        .and_then(|animation| animation.channels().next())
        .ok_or("a channel")?;
    assert_eq!(
        node_name(&glb, channel.target().node().index()),
        Some("cube_mesh"),
        "the node bone 1 drives"
    );
    Ok(())
}

#[test]
fn a_model_without_animations_has_none() -> TestResult {
    let (glb, summary) = glb(&skin()?, &[])?;

    assert_eq!(glb.document.animations().len(), 0, "no animations");
    assert!(summary.animations.is_empty(), "no summaries");
    Ok(())
}
