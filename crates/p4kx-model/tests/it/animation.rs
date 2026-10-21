//! Reading `.caf` files and `.dba` databases into tracks of rotation and translation keys.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_model::{Animation, Dba, Keys, Track};
use p4kx_testkit::{
    TestAnimation, TestDba, TestPositionFormat, TestRotationFormat, TestTimes, TestTrack,
};

type TestResult = Result<(), Box<dyn Error>>;

const HALF_TURN_Z: [f32; 4] = [0.0, 0.0, 1.0, 0.0];
const QUARTER_TURN_Z: [f32; 4] = [0.0, 0.0, 0.707_106_77, 0.707_106_77];
const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

fn read_caf(animation: &TestAnimation) -> Result<Animation, Box<dyn Error>> {
    let file = ChunkFile::parse(animation.caf())?;
    Ok(Animation::read_caf("walk.caf", &file)?)
}

fn read_dba(dba: &TestDba) -> Result<Dba, Box<dyn Error>> {
    let file = ChunkFile::parse(dba.bytes())?;
    Ok(Dba::read("pack.dba", &file)?)
}

fn only_track(animation: &Animation) -> Result<&Track, Box<dyn Error>> {
    animation
        .tracks
        .first()
        .ok_or_else(|| "a track".to_owned().into())
}

fn rotations(track: &Track) -> Result<&Keys<[f32; 4]>, Box<dyn Error>> {
    track
        .rotations
        .as_ref()
        .ok_or_else(|| "rotation keys".to_owned().into())
}

fn translations(track: &Track) -> Result<&Keys<[f32; 3]>, Box<dyn Error>> {
    track
        .translations
        .as_ref()
        .ok_or_else(|| "translation keys".to_owned().into())
}

fn assert_close(actual: &[f32], expected: &[f32], tolerance: f32, what: &str) {
    assert_eq!(actual.len(), expected.len(), "{what}: length");
    for (a, e) in actual.iter().zip(expected) {
        assert!(
            (a - e).abs() <= tolerance,
            "{what}: {actual:?} is not {expected:?}"
        );
    }
}

fn dot(left: [f32; 4], right: [f32; 4]) -> f32 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

#[test]
fn a_caf_has_its_name_frame_rate_and_end_frame() -> TestResult {
    let animation = TestAnimation::new("walk.caf", 30).track(
        TestTrack::new(7)
            .rotations(&[(0, IDENTITY), (45, QUARTER_TURN_Z)])
            .positions(&[(0, [0.0; 3]), (60, [1.0, 2.0, 3.0])]),
    );

    let read = read_caf(&animation)?;

    assert_eq!(read.name, "walk", "name without extension");
    assert_eq!(read.path, "walk.caf", "the file name");
    assert_eq!(read.frames_per_second, 30, "frame rate");
    assert_eq!(read.end_frame, 60, "end frame is the last key");
    assert!(
        (read.duration_seconds() - 2.0).abs() < 1e-6,
        "60 frames at 30 fps"
    );
    assert!(!read.additive, "not additive");
    assert_eq!(read.tracks.len(), 1, "one track");
    assert_eq!(read.key_count(), 4, "two rotations and two translations");
    Ok(())
}

#[test]
fn small_tree_rotations_decode_to_the_unit_quaternions_they_encode() -> TestResult {
    let keys = [
        (0_u16, IDENTITY),
        (10, QUARTER_TURN_Z),
        (20, [0.5, 0.5, 0.5, 0.5]),
        (30, [-0.6, 0.0, 0.8, 0.0]),
        (40, [0.0, -1.0, 0.0, 0.0]),
        (50, [0.1, 0.2, -0.3, 0.927_361_85]),
    ];
    for format in [
        TestRotationFormat::SmallTree,
        TestRotationFormat::SmallTree64,
    ] {
        let animation = TestAnimation::new("a.caf", 30)
            .track(TestTrack::new(1).rotation_format(format).rotations(&keys));

        let read = read_caf(&animation)?;

        let decoded = rotations(only_track(&read)?)?;
        assert_eq!(
            decoded.frames,
            [0, 10, 20, 30, 40, 50],
            "{format:?}: frames"
        );
        for (value, (_, expected)) in decoded.values.iter().zip(&keys) {
            let length = value.iter().map(|v| v * v).sum::<f32>().sqrt();
            assert!(
                (length - 1.0).abs() < 1e-5,
                "{format:?}: unit length: {value:?}"
            );
            assert!(
                dot(*value, *expected).abs() > 1.0 - 1e-4,
                "{format:?}: {value:?} is not {expected:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn float_rotations_are_normalised() -> TestResult {
    let animation = TestAnimation::new("a.caf", 30).track(
        TestTrack::new(1)
            .rotation_format(TestRotationFormat::Float)
            .rotations(&[(0, [0.0, 0.0, 0.0, 2.0]), (5, [0.0, 3.0, 0.0, 0.0])]),
    );

    let read = read_caf(&animation)?;

    let decoded = rotations(only_track(&read)?)?;
    assert_close(&decoded.values[0], &IDENTITY, 1e-6, "scaled to unit length");
    assert_close(
        &decoded.values[1],
        &[0.0, 1.0, 0.0, 0.0],
        1e-6,
        "scaled to unit length",
    );
    Ok(())
}

#[test]
fn consecutive_rotations_lie_in_one_hemisphere() -> TestResult {
    let animation = TestAnimation::new("a.caf", 30).track(
        TestTrack::new(1)
            .rotation_format(TestRotationFormat::Float)
            .rotations(&[
                (0, QUARTER_TURN_Z),
                (1, QUARTER_TURN_Z.map(|v| -v)),
                (2, HALF_TURN_Z),
            ]),
    );

    let read = read_caf(&animation)?;

    let decoded = rotations(only_track(&read)?)?;
    for pair in decoded.values.windows(2) {
        assert!(
            matches!(pair, [a, b] if dot(*a, *b) >= 0.0),
            "{pair:?} must not point apart"
        );
    }
    Ok(())
}

#[test]
fn every_position_format_gives_the_positions_it_encodes() -> TestResult {
    let keys = [
        (0_u16, [0.0, -1.5, 10.0]),
        (4, [0.25, -1.5, 12.5]),
        (9, [3.5, -0.5, 9.0]),
    ];
    for (format, tolerance) in [
        (TestPositionFormat::Float, 0.0),
        (TestPositionFormat::Interleaved, 1e-3),
        (TestPositionFormat::Planar, 1e-3),
    ] {
        let animation = TestAnimation::new("a.caf", 24)
            .track(TestTrack::new(1).position_format(format).positions(&keys));

        let read = read_caf(&animation)?;

        let decoded = translations(only_track(&read)?)?;
        assert_eq!(decoded.frames, [0, 4, 9], "{format:?}: frames");
        for (value, (_, expected)) in decoded.values.iter().zip(&keys) {
            assert_close(value, expected, tolerance, &format!("{format:?}"));
        }
    }
    Ok(())
}

#[test]
fn planar_positions_keep_axes_that_never_change() -> TestResult {
    let keys = [(0_u16, [1.0, 2.0, 3.0]), (8, [1.0, 2.5, 3.0])];
    let animation = TestAnimation::new("a.caf", 30).track(
        TestTrack::new(1)
            .position_format(TestPositionFormat::Planar)
            .positions(&keys),
    );

    let read = read_caf(&animation)?;

    let decoded = translations(only_track(&read)?)?;
    assert_close(&decoded.values[0], &[1.0, 2.0, 3.0], 1e-5, "first key");
    assert_close(&decoded.values[1], &[1.0, 2.5, 3.0], 1e-4, "second key");
    Ok(())
}

#[test]
fn every_time_format_gives_the_same_frames() -> TestResult {
    let frames = [2_u16, 3, 9, 20, 21, 77];
    let keys: Vec<(u16, [f32; 3])> = frames.iter().map(|&frame| (frame, [0.0; 3])).collect();
    for times in [TestTimes::Bytes, TestTimes::Shorts, TestTimes::Bitmap] {
        let animation =
            TestAnimation::new("a.caf", 30).track(TestTrack::new(1).times(times).positions(&keys));

        let read = read_caf(&animation)?;

        let decoded = translations(only_track(&read)?)?;
        assert_eq!(decoded.frames, frames, "{times:?}");
    }
    Ok(())
}

#[test]
fn frames_beyond_a_byte_need_the_wider_time_formats() -> TestResult {
    let keys = [
        (100_u16, [0.0; 3]),
        (300, [1.0, 0.0, 0.0]),
        (65_000, [2.0; 3]),
    ];
    for times in [TestTimes::Shorts, TestTimes::Bitmap] {
        let animation =
            TestAnimation::new("a.caf", 30).track(TestTrack::new(1).times(times).positions(&keys));

        let read = read_caf(&animation)?;

        let decoded = translations(only_track(&read)?)?;
        assert_eq!(decoded.frames, [100, 300, 65_000], "{times:?}");
        assert_eq!(read.end_frame, 65_000, "{times:?}: end frame");
    }
    Ok(())
}

#[test]
fn a_track_with_one_channel_leaves_the_other_empty() -> TestResult {
    let animation = TestAnimation::new("a.caf", 30)
        .track(TestTrack::new(1).rotations(&[(0, IDENTITY)]))
        .track(TestTrack::new(2).positions(&[(0, [1.0; 3])]));

    let read = read_caf(&animation)?;

    let [rotating, moving] = read.tracks.as_slice() else {
        return Err("two tracks".into());
    };
    assert!(rotating.translations.is_none(), "no translations");
    assert!(moving.rotations.is_none(), "no rotations");
    assert_eq!(rotating.controller_id, 1, "first controller");
    assert_eq!(moving.controller_id, 2, "second controller");
    Ok(())
}

#[test]
fn tracks_without_keys_are_dropped() -> TestResult {
    let animation = TestAnimation::new("a.caf", 30)
        .track(TestTrack::new(1))
        .track(TestTrack::new(2).rotations(&[(0, IDENTITY)]));

    let read = read_caf(&animation)?;

    assert_eq!(read.tracks.len(), 1, "only the track with keys");
    assert_eq!(only_track(&read)?.controller_id, 2, "which one");
    Ok(())
}

#[test]
fn the_additive_flag_is_read() -> TestResult {
    let animation = TestAnimation::new("a.caf", 30)
        .additive()
        .track(TestTrack::new(1).rotations(&[(0, IDENTITY)]));

    assert!(read_caf(&animation)?.additive, "additive");
    Ok(())
}

#[test]
fn a_database_has_its_animations_in_order_with_names_and_rates() -> TestResult {
    let dba = TestDba::new()
        .animation(
            TestAnimation::new("animations/human/walk/forward.caf", 30)
                .track(TestTrack::new(1).rotations(&[(0, IDENTITY), (10, QUARTER_TURN_Z)])),
        )
        .animation(
            TestAnimation::new("animations/human/walk/back.caf", 60)
                .additive()
                .track(TestTrack::new(2).positions(&[(0, [0.0; 3]), (90, [0.0, 1.0, 0.0])]))
                .track(TestTrack::new(3).rotations(&[(5, HALF_TURN_Z)])),
        );

    let read = read_dba(&dba)?;

    assert!(read.skipped.is_empty(), "nothing skipped");
    let names: Vec<&str> = read.animations.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["forward", "back"], "stems in database order");
    let paths: Vec<&str> = read.animations.iter().map(|a| a.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "animations/human/walk/forward.caf",
            "animations/human/walk/back.caf"
        ],
        "paths as stored"
    );
    let [forward, back] = read.animations.as_slice() else {
        return Err("two animations".into());
    };
    assert_eq!(forward.frames_per_second, 30, "forward rate");
    assert_eq!(forward.end_frame, 10, "forward end frame");
    assert_eq!(back.frames_per_second, 60, "back rate");
    assert_eq!(back.end_frame, 90, "back end frame");
    assert!(back.additive && !forward.additive, "additive flags");
    assert_eq!(back.tracks.len(), 2, "back tracks");
    assert!(
        (back.duration_seconds() - 1.5).abs() < 1e-6,
        "90 frames at 60 fps"
    );
    Ok(())
}

#[test]
fn a_database_pool_behind_the_metadata_chunk_is_reached() -> TestResult {
    let dba = TestDba::new()
        .animation(
            TestAnimation::new("a.caf", 30).track(
                TestTrack::new(1)
                    .rotations(&[(0, IDENTITY), (20, QUARTER_TURN_Z)])
                    .positions(&[(0, [0.0; 3]), (20, [4.0, 0.0, 0.0])]),
            ),
        )
        .animation(
            TestAnimation::new("b.caf", 30).track(TestTrack::new(2).rotations(&[(3, HALF_TURN_Z)])),
        )
        .pool_after_metadata();

    let read = read_dba(&dba)?;

    assert!(read.skipped.is_empty(), "nothing skipped");
    let [first, second] = read.animations.as_slice() else {
        return Err("two animations".into());
    };
    assert_close(
        &translations(only_track(first)?)?.values[1],
        &[4.0, 0.0, 0.0],
        1e-3,
        "translation behind the chunk span",
    );
    assert_eq!(rotations(only_track(second)?)?.frames, [3], "second frames");
    Ok(())
}

#[test]
fn an_animation_with_an_unknown_key_format_is_skipped_and_the_rest_are_read() -> TestResult {
    let dba = TestDba::new()
        .animation(
            TestAnimation::new("bad.caf", 30).track(
                TestTrack::new(1)
                    .rotation_code(0x99)
                    .rotations(&[(0, IDENTITY)]),
            ),
        )
        .animation(
            TestAnimation::new("good.caf", 30).track(TestTrack::new(2).rotations(&[(0, IDENTITY)])),
        );

    let read = read_dba(&dba)?;

    let names: Vec<&str> = read.animations.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["good"], "the readable one");
    let [skipped] = read.skipped.as_slice() else {
        return Err("one skipped".into());
    };
    assert_eq!(skipped.name, "bad", "the skipped one");
    assert_eq!(
        skipped.reason,
        p4kx_model::CorruptAnimation::UnknownFormat {
            kind: "rotation",
            code: 0x99
        },
        "reason"
    );
    Ok(())
}
