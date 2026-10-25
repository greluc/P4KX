// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Reading the animations next to a model within a bound and a cancellation.

use std::error::Error;

use p4kx_extraction::{
    AnimationBounds, AnimationSet, Archive, CancelToken, ConversionError, PathIndex,
};
use p4kx_testkit::{ArchiveBuilder, TestAnimation, TestEntry, TestSkeleton, TestTrack};

type TestResult = Result<(), Box<dyn Error>>;

const ROOMY: AnimationBounds = AnimationBounds {
    files: 64,
    bytes: 64 << 20,
};

fn animation(name: &str) -> TestAnimation {
    TestAnimation::new(name, 30).track(
        TestTrack::new(TestSkeleton::controller_id(1))
            .rotations(&[(0, [0.0, 0.0, 0.0, 1.0]), (30, [0.0, 0.0, 0.0, 1.0])]),
    )
}

fn archive(dir: &tempfile::TempDir, files: usize) -> Result<Archive, Box<dyn Error>> {
    let path = dir.path().join("Data.p4k");
    let mut builder = ArchiveBuilder::new();
    for index in 0..files {
        let name = format!("clip{index}.caf");
        builder = builder.entry(TestEntry::stored(
            format!("Data\\Objects\\{name}"),
            animation(&name).caf(),
        ));
    }
    builder
        .entry(TestEntry::stored("Data\\Objects\\door.cga", vec![0; 4]))
        .write_to(&path)?;
    Ok(Archive::open_with_key(
        &path,
        Some(p4kx_testkit::test_key()),
    )?)
}

#[test]
fn the_animations_within_the_bounds_are_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(&dir, 3)?;
    let index = PathIndex::new(&archive);

    let set = AnimationSet::from_archive_bounded(
        &index,
        "Data\\Objects\\door.cga",
        ROOMY,
        &CancelToken::new(),
    )?
    .ok_or("over the bounds")?;

    assert_eq!(set.animations.len(), 3, "all three");
    assert_eq!(set.files, 3, "three files");
    Ok(())
}

#[test]
fn more_files_than_the_bound_allows_are_not_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(&dir, 3)?;
    let index = PathIndex::new(&archive);
    let bounds = AnimationBounds { files: 2, ..ROOMY };

    let set = AnimationSet::from_archive_bounded(
        &index,
        "Data\\Objects\\door.cga",
        bounds,
        &CancelToken::new(),
    )?;

    assert!(set.is_none(), "over the file bound");
    Ok(())
}

#[test]
fn more_bytes_than_the_bound_allows_are_not_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(&dir, 3)?;
    let index = PathIndex::new(&archive);
    let bounds = AnimationBounds { bytes: 16, ..ROOMY };

    let set = AnimationSet::from_archive_bounded(
        &index,
        "Data\\Objects\\door.cga",
        bounds,
        &CancelToken::new(),
    )?;

    assert!(set.is_none(), "over the byte bound");
    Ok(())
}

#[test]
fn a_cancelled_read_stops_with_the_cancellation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(&dir, 3)?;
    let index = PathIndex::new(&archive);
    let cancel = CancelToken::new();
    cancel.cancel();

    let result =
        AnimationSet::from_archive_bounded(&index, "Data\\Objects\\door.cga", ROOMY, &cancel);

    assert!(
        matches!(result, Err(ConversionError::Cancelled)),
        "cancelled, not {result:?}"
    );
    Ok(())
}
