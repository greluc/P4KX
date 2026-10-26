// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Reading the headers of `.caf` files and `.dba` databases without decoding a key.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_model::{
    Animation, AnimationHeader, Bone, CompanionError, CompanionPath, Dba, DbaHeaders, Model,
};
use p4kx_testkit::{TestAnimation, TestDba, TestModel, TestSkeleton, TestTrack};

type TestResult = Result<(), Box<dyn Error>>;

const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

fn bones(count: u16) -> Result<Vec<Bone>, Box<dyn Error>> {
    let rig = TestModel::cube()
        .helpers_only()
        .with_skeleton(&TestSkeleton::chain(count));
    let file = ChunkFile::parse(rig.cgf())?;
    let mut companions =
        |_: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> { Ok(None) };
    Ok(Model::read("rig.chr", &file, &mut companions)?.bones)
}

fn sample() -> TestAnimation {
    TestAnimation::new("animations/walk/forward.caf", 30)
        .additive()
        .track(
            TestTrack::new(TestSkeleton::controller_id(1))
                .rotations(&[(0, IDENTITY), (10, IDENTITY), (45, IDENTITY)])
                .positions(&[(0, [0.0; 3]), (45, [1.0, 2.0, 3.0])]),
        )
        .track(TestTrack::new(TestSkeleton::controller_id(7)).rotations(&[(0, IDENTITY)]))
        .track(TestTrack::new(TestSkeleton::controller_id(1)).rotations(&[(0, IDENTITY)]))
        .track(TestTrack::new(9))
}

#[test]
fn a_caf_header_says_what_the_decoded_animation_says() -> TestResult {
    let file = ChunkFile::parse(sample().caf())?;
    let header = AnimationHeader::read_caf("forward.caf", &file)?;
    let full = Animation::read_caf("forward.caf", &file)?;

    assert_eq!(header.name, full.name, "name");
    assert_eq!(header.path, full.path, "path");
    assert_eq!(header.frames_per_second, full.frames_per_second, "rate");
    assert_eq!(header.end_frame, full.end_frame, "end frame");
    assert_eq!(header.additive, full.additive, "additive");
    assert_eq!(header.keys, full.key_count(), "keys");
    let ids: Vec<u32> = full
        .tracks
        .iter()
        .map(|track| track.controller_id)
        .collect();
    assert_eq!(
        header.controller_ids, ids,
        "tracks without keys are dropped"
    );
    assert!(
        (header.duration_seconds() - full.duration_seconds()).abs() < f32::EPSILON,
        "duration"
    );
    Ok(())
}

#[test]
fn the_tracks_a_header_binds_are_the_tracks_the_animation_binds() -> TestResult {
    let bones = bones(4)?;
    let file = ChunkFile::parse(sample().caf())?;
    let header = AnimationHeader::read_caf("forward.caf", &file)?;
    let full = Animation::read_caf("forward.caf", &file)?;

    assert_eq!(
        header.bound_tracks(&bones),
        full.bind(&bones).bound.len(),
        "a second track for a bone and a bone the skeleton lacks are unbound"
    );
    assert_eq!(header.bound_tracks(&[]), 0, "no bones");
    Ok(())
}

#[test]
fn a_database_header_lists_the_animations_in_order() -> TestResult {
    let dba = TestDba::new().animation(sample()).animation(
        TestAnimation::new("animations/walk/back.caf", 60)
            .track(TestTrack::new(3).rotations(&[(0, IDENTITY), (60, IDENTITY)])),
    );
    let file = ChunkFile::parse(dba.bytes())?;

    let headers = DbaHeaders::read("pack.dba", &file)?;
    let full = Dba::read("pack.dba", &file)?;

    assert_eq!(headers.animations.len(), full.animations.len(), "count");
    assert!(headers.skipped.is_empty(), "nothing skipped");
    for (header, full) in headers.animations.iter().zip(&full.animations) {
        assert_eq!(header.name, full.name, "name");
        assert_eq!(header.path, full.path, "path");
        assert_eq!(header.end_frame, full.end_frame, "end frame");
        assert_eq!(header.keys, full.key_count(), "keys of {}", full.name);
        assert_eq!(header.controller_ids.len(), full.tracks.len(), "tracks");
    }
    Ok(())
}

#[test]
fn a_header_does_not_notice_a_key_format_the_reader_does_not_know() -> TestResult {
    let dba = TestDba::new().animation(
        TestAnimation::new("bad.caf", 30).track(
            TestTrack::new(1)
                .rotation_code(0x99)
                .rotations(&[(0, IDENTITY)]),
        ),
    );
    let file = ChunkFile::parse(dba.bytes())?;

    let headers = DbaHeaders::read("pack.dba", &file)?;
    let full = Dba::read("pack.dba", &file)?;

    assert_eq!(headers.animations.len(), 1, "listed from its header");
    assert_eq!(full.skipped.len(), 1, "skipped when decoded");
    Ok(())
}

#[test]
fn a_header_refuses_what_the_decoder_refuses_before_the_keys() -> TestResult {
    let file = ChunkFile::parse(TestAnimation::new("zero.caf", 0).caf())?;

    assert!(
        AnimationHeader::read_caf("zero.caf", &file).is_err(),
        "a frame rate of zero"
    );
    Ok(())
}
