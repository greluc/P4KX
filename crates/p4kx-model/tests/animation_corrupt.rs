//! Corrupt, unsupported or hostile animation files give typed errors, never a panic.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_model::{Animation, AnimationError, CorruptAnimation, Dba, MAX_ANIMATION_KEYS};
use p4kx_testkit::{TestAnimation, TestChunkFile, TestDba, TestTimes, TestTrack};
use proptest::collection::vec;
use proptest::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const ANIM_INFO: u32 = 0x4733_C6ED;
const CAF_DATA: u32 = 0xA949_6CB5;
const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

fn caf_error(bytes: Vec<u8>) -> Result<AnimationError, Box<dyn Error>> {
    let file = ChunkFile::parse(bytes)?;
    Animation::read_caf("a.caf", &file)
        .err()
        .ok_or_else(|| "an error".to_owned().into())
}

fn corrupt_reason(error: AnimationError) -> Result<CorruptAnimation, Box<dyn Error>> {
    match error {
        AnimationError::Corrupt { source, .. } => Ok(source),
        other => Err(format!("expected a corrupt chunk, got {other:?}").into()),
    }
}

fn animation() -> TestAnimation {
    TestAnimation::new("a.caf", 30).track(
        TestTrack::new(1)
            .rotations(&[(0, IDENTITY), (9, [0.0, 0.0, 1.0, 0.0])])
            .positions(&[(0, [0.0; 3]), (9, [1.0, 2.0, 3.0])]),
    )
}

#[test]
fn a_crch_file_is_not_supported() -> TestResult {
    let error = caf_error(TestChunkFile::crch().bytes())?;

    assert!(
        matches!(error, AnimationError::UnsupportedContainer { .. }),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_caf_without_its_data_chunk_is_named() -> TestResult {
    let bytes = TestChunkFile::ivo()
        .chunk(ANIM_INFO, 0x901, animation().anim_info())
        .bytes();

    let error = caf_error(bytes)?;

    assert!(
        matches!(
            error,
            AnimationError::MissingChunk {
                chunk: "CafData",
                ..
            }
        ),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_chunk_with_an_unknown_version_is_refused() -> TestResult {
    let bytes = TestChunkFile::ivo()
        .chunk(ANIM_INFO, 0x777, animation().anim_info())
        .chunk(CAF_DATA, 0x900, animation().caf_data())
        .bytes();

    let error = caf_error(bytes)?;

    assert!(
        matches!(
            error,
            AnimationError::UnsupportedVersion {
                chunk: "AnimInfo",
                version: 0x777,
                ..
            }
        ),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_frame_rate_of_zero_is_corrupt() -> TestResult {
    let error = caf_error(TestAnimation::new("a.caf", 0).caf())?;

    assert_eq!(
        corrupt_reason(error)?,
        CorruptAnimation::FrameRate,
        "reason"
    );
    Ok(())
}

#[test]
fn an_unknown_rotation_format_is_reported_not_decoded() -> TestResult {
    let bad = TestAnimation::new("a.caf", 30).track(
        TestTrack::new(1)
            .rotation_code(0x81)
            .rotations(&[(0, IDENTITY)]),
    );

    let reason = corrupt_reason(caf_error(bad.caf())?)?;

    assert_eq!(
        reason,
        CorruptAnimation::UnknownFormat {
            kind: "rotation",
            code: 0x81
        },
        "reason"
    );
    Ok(())
}

#[test]
fn key_times_that_do_not_increase_are_corrupt() -> TestResult {
    let bad = TestAnimation::new("a.caf", 30)
        .track(TestTrack::new(7).positions(&[(5, [0.0; 3]), (3, [1.0; 3])]));

    let reason = corrupt_reason(caf_error(bad.caf())?)?;

    assert_eq!(
        reason,
        CorruptAnimation::TimesNotIncreasing { controller_id: 7 },
        "reason"
    );
    Ok(())
}

#[test]
fn a_bitmap_that_marks_fewer_frames_than_the_track_has_keys_is_corrupt() -> TestResult {
    let good = TestAnimation::new("a.caf", 30)
        .track(
            TestTrack::new(7)
                .times(TestTimes::Bitmap)
                .positions(&[(2, [0.0; 3]), (6, [1.0; 3])]),
        )
        .caf();
    let head = good
        .windows(4)
        .position(|window| window == [2, 0, 6, 0])
        .ok_or("the bitmap header")?;
    let mut bad = good;
    let bitmap = bad.get_mut(head + 4).ok_or("the bitmap byte")?;
    assert_eq!(*bitmap, 0b0001_0001, "frames 2 and 6 relative to frame 2");
    *bitmap = 0b0000_0001;

    let reason = corrupt_reason(caf_error(bad)?)?;

    assert_eq!(
        reason,
        CorruptAnimation::BitmapCount {
            controller_id: 7,
            expected: 2,
            found: 1
        },
        "reason"
    );
    Ok(())
}

#[test]
fn a_caf_cut_short_is_truncated() -> TestResult {
    let mut bytes = animation().caf();
    bytes.truncate(bytes.len() - 5);

    let reason = corrupt_reason(caf_error(bytes)?)?;

    assert!(
        matches!(reason, CorruptAnimation::Truncated(_)),
        "{reason:?}"
    );
    Ok(())
}

#[test]
fn a_block_with_the_wrong_signature_is_refused() -> TestResult {
    let mut data = animation().caf_data();
    data.get_mut(..4)
        .ok_or("the signature")?
        .copy_from_slice(b"#dba");
    let bytes = TestChunkFile::ivo()
        .chunk(ANIM_INFO, 0x901, animation().anim_info())
        .chunk(CAF_DATA, 0x900, data)
        .bytes();

    let reason = corrupt_reason(caf_error(bytes)?)?;

    assert_eq!(
        reason,
        CorruptAnimation::Signature {
            expected: *b"#caf",
            found: *b"#dba"
        },
        "reason"
    );
    Ok(())
}

#[test]
fn an_animation_that_declares_too_many_keys_is_refused_before_anything_is_allocated() -> TestResult
{
    let controllers: u16 = 65;
    let mut data = Vec::new();
    data.extend_from_slice(b"#caf");
    data.extend_from_slice(&controllers.to_le_bytes());
    data.extend_from_slice(&0xFFFF_u16.to_le_bytes());
    data.extend_from_slice(&0_u32.to_le_bytes());
    for hash in 0..u32::from(controllers) {
        data.extend_from_slice(&hash.to_le_bytes());
    }
    for _ in 0..controllers {
        for format in [0x8240_u16, 0xC240] {
            data.extend_from_slice(&u16::MAX.to_le_bytes());
            data.extend_from_slice(&format.to_le_bytes());
            data.extend_from_slice(&[0; 8]);
        }
    }
    let bytes = TestChunkFile::ivo()
        .chunk(ANIM_INFO, 0x901, animation().anim_info())
        .chunk(CAF_DATA, 0x900, data)
        .bytes();

    let reason = corrupt_reason(caf_error(bytes)?)?;

    assert_eq!(
        reason,
        CorruptAnimation::TooManyKeys {
            keys: 65 * 2 * u64::from(u16::MAX),
            limit: MAX_ANIMATION_KEYS
        },
        "reason"
    );
    Ok(())
}

#[test]
fn a_database_without_its_metadata_is_named() -> TestResult {
    let bytes = TestChunkFile::ivo()
        .chunk(0x194F_BC50, 0x900, vec![0; 8])
        .bytes();
    let file = ChunkFile::parse(bytes)?;

    let error = Dba::read("a.dba", &file).err().ok_or("an error")?;

    assert!(
        matches!(
            error,
            AnimationError::MissingChunk {
                chunk: "DbaMetadata",
                ..
            }
        ),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_database_that_lists_more_animations_than_its_metadata_holds_is_truncated() -> TestResult {
    let mut metadata = Vec::new();
    metadata.extend_from_slice(&1_000_000_u32.to_le_bytes());
    metadata.extend_from_slice(&[0; 60]);
    let bytes = TestChunkFile::ivo()
        .chunk(0x194F_BC50, 0x900, vec![0; 8])
        .chunk(0xF735_1608, 0x902, metadata)
        .bytes();
    let file = ChunkFile::parse(bytes)?;

    let error = Dba::read("a.dba", &file).err().ok_or("an error")?;

    assert!(
        matches!(corrupt_reason(error)?, CorruptAnimation::Truncated(_)),
        "truncated"
    );
    Ok(())
}

fn check(bytes: &[u8]) -> Result<(), TestCaseError> {
    if let Ok(file) = ChunkFile::parse(bytes) {
        let caf = Animation::read_caf("a.caf", &file);
        let dba = Dba::read("a.dba", &file);
        if let Ok(animation) = caf {
            prop_assert!(animation.duration_seconds().is_finite(), "duration");
            for track in &animation.tracks {
                if let Some(keys) = &track.rotations {
                    prop_assert_eq!(keys.frames.len(), keys.values.len(), "rotation keys");
                    prop_assert!(
                        keys.frames
                            .windows(2)
                            .all(|pair| matches!(pair, [a, b] if a < b)),
                        "increasing"
                    );
                    prop_assert!(
                        keys.values.iter().flatten().all(|v| v.is_finite()),
                        "finite"
                    );
                }
                if let Some(keys) = &track.translations {
                    prop_assert_eq!(keys.frames.len(), keys.values.len(), "translation keys");
                    prop_assert!(
                        keys.values.iter().flatten().all(|v| v.is_finite()),
                        "finite"
                    );
                }
            }
        }
        drop(dba);
    }
    Ok(())
}

fn mutated(base: &[u8], flips: &[(usize, u8)], cut: usize) -> Vec<u8> {
    let mut bytes = base.to_vec();
    let len = bytes.len();
    for &(at, value) in flips {
        if let Some(byte) = bytes.get_mut(at % len) {
            *byte = value;
        }
    }
    bytes.truncate(len - cut % len);
    bytes
}

proptest! {
    #[test]
    fn arbitrary_bytes_never_panic(bytes in vec(any::<u8>(), 0..256)) {
        check(&bytes)?;
    }

    #[test]
    fn mutated_cafs_never_panic(
        flips in vec((any::<usize>(), any::<u8>()), 0..12),
        cut in any::<usize>(),
    ) {
        let base = animation().caf();
        check(&mutated(&base, &flips, cut % 8))?;
    }

    #[test]
    fn mutated_databases_never_panic(
        flips in vec((any::<usize>(), any::<u8>()), 0..12),
        cut in any::<usize>(),
        after in any::<bool>(),
    ) {
        let mut dba = TestDba::new().animation(animation()).animation(
            TestAnimation::new("b.caf", 60).track(TestTrack::new(2).times(TestTimes::Bitmap).positions(&[(1, [0.0; 3]), (40, [1.0; 3])])),
        );
        if after {
            dba = dba.pool_after_metadata();
        }
        let base = dba.bytes();
        check(&mutated(&base, &flips, cut % 8))?;
    }
}
