// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Matching the tracks of an animation to the bones of a skeleton by controller ID.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_model::{Animation, Bone, CompanionError, CompanionPath, Model};
use p4kx_testkit::{TestAnimation, TestModel, TestSkeleton, TestTrack};

type TestResult = Result<(), Box<dyn Error>>;

fn rig_bones(count: u16) -> Result<Vec<Bone>, Box<dyn Error>> {
    let rig = TestModel::cube()
        .helpers_only()
        .with_skeleton(&TestSkeleton::chain(count));
    let file = ChunkFile::parse(rig.cgf())?;
    let mut companions =
        |_: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> { Ok(None) };
    Ok(Model::read("rig.chr", &file, &mut companions)?.bones)
}

fn animation(tracks: Vec<TestTrack>) -> Result<Animation, Box<dyn Error>> {
    let mut source = TestAnimation::new("a.caf", 30);
    for track in tracks {
        source = source.track(track);
    }
    let file = ChunkFile::parse(source.caf())?;
    Ok(Animation::read_caf("a.caf", &file)?)
}

fn track(bone: usize) -> TestTrack {
    TestTrack::new(TestSkeleton::controller_id(bone)).positions(&[(0, [0.0; 3])])
}

#[test]
fn tracks_go_to_the_bones_with_their_controller_id() -> TestResult {
    let bones = rig_bones(3)?;
    let read = animation(vec![track(2), track(0)])?;

    let bound = read.bind(&bones);

    let indices: Vec<usize> = bound.bound.iter().map(|&(bone, _)| bone).collect();
    assert_eq!(indices, [2, 0], "bone indices in track order");
    assert!(bound.unbound.is_empty(), "every track found its bone");
    Ok(())
}

#[test]
fn tracks_of_bones_the_skeleton_lacks_are_reported_by_controller_id() -> TestResult {
    let bones = rig_bones(2)?;
    let read = animation(vec![track(1), track(5), track(9)])?;

    let bound = read.bind(&bones);

    assert_eq!(bound.bound.len(), 1, "one track found a bone");
    assert_eq!(
        bound.unbound,
        [
            TestSkeleton::controller_id(5),
            TestSkeleton::controller_id(9)
        ],
        "the others are unbound"
    );
    Ok(())
}

#[test]
fn a_second_track_for_the_same_bone_is_unbound() -> TestResult {
    let bones = rig_bones(2)?;
    let read = animation(vec![track(1), track(1)])?;

    let bound = read.bind(&bones);

    assert_eq!(bound.bound.len(), 1, "the first track is kept");
    assert_eq!(
        bound.unbound,
        [TestSkeleton::controller_id(1)],
        "the second is reported"
    );
    Ok(())
}

#[test]
fn an_animation_binds_to_no_bones_of_an_empty_skeleton() -> TestResult {
    let read = animation(vec![track(0)])?;

    let bound = read.bind(&[]);

    assert!(bound.bound.is_empty(), "nothing bound");
    assert_eq!(bound.unbound.len(), 1, "the track is reported");
    Ok(())
}
