// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Listing the animations next to a model from their headers.

use std::error::Error;

use p4kx_extraction::{
    AnimationChoice, AnimationListing, AnimationSet, Archive, ConversionError, PathIndex,
};
use p4kx_testkit::{ArchiveBuilder, TestAnimation, TestDba, TestEntry, TestSkeleton, TestTrack};

type TestResult = Result<(), Box<dyn Error>>;

const MODEL: &str = "Data\\Objects\\door.cga";
const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

fn animation(name: &str) -> TestAnimation {
    TestAnimation::new(name, 30).track(
        TestTrack::new(TestSkeleton::controller_id(1))
            .rotations(&[(0, IDENTITY), (30, IDENTITY)])
            .positions(&[(0, [0.0; 3]), (30, [1.0; 3])]),
    )
}

fn archive(dir: &tempfile::TempDir) -> Result<Archive, Box<dyn Error>> {
    let path = dir.path().join("Data.p4k");
    let pack = TestDba::new()
        .animation(animation("pack/open.caf"))
        .animation(animation("pack/close.caf").additive());
    ArchiveBuilder::new()
        .entry(TestEntry::stored(
            "Data\\Objects\\open.caf",
            animation("open.caf").caf(),
        ))
        .entry(TestEntry::stored(
            "Data\\Objects\\lift_add.caf",
            animation("lift_add.caf").additive().caf(),
        ))
        .entry(TestEntry::stored("Data\\Objects\\pack.dba", pack.bytes()))
        .entry(TestEntry::stored(
            "Data\\Objects\\broken.caf",
            vec![1, 2, 3],
        ))
        .entry(TestEntry::stored(MODEL, vec![0; 4]))
        .write_to(&path)?;
    Ok(Archive::open_with_key(
        &path,
        Some(p4kx_testkit::test_key()),
    )?)
}

#[test]
fn the_listing_finds_what_reading_in_full_finds() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(&dir)?;
    let index = PathIndex::new(&archive);

    let listing = AnimationListing::from_archive(&index, MODEL, &[])?;
    let set = AnimationSet::from_archive(&index, MODEL, &AnimationChoice::All, &[])?;

    assert_eq!(listing.files, set.files, "files");
    assert_eq!(listing.origins, set.origins, "origins");
    assert_eq!(listing.unread, set.unread, "additive and unreadable ones");
    assert_eq!(listing.animations.len(), set.animations.len(), "count");
    for (header, full) in listing.animations.iter().zip(&set.animations) {
        assert_eq!(header.name, full.name, "name");
        assert_eq!(header.keys, full.key_count(), "keys of {}", full.name);
        assert_eq!(
            header.end_frame, full.end_frame,
            "end frame of {}",
            full.name
        );
        assert_eq!(header.controller_ids.len(), full.tracks.len(), "tracks");
    }
    Ok(())
}

#[test]
fn the_listing_reports_an_additive_animation_and_a_broken_file_as_unread() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(&dir)?;
    let index = PathIndex::new(&archive);

    let listing = AnimationListing::from_archive(&index, MODEL, &[])?;

    let names: Vec<&str> = listing
        .animations
        .iter()
        .map(|header| header.name.as_str())
        .collect();
    assert_eq!(names, ["open", "open"], "the plain ones");
    let unread: Vec<&str> = listing
        .unread
        .iter()
        .map(|unread| unread.name.as_deref().unwrap_or(unread.source.as_str()))
        .collect();
    assert!(
        unread.contains(&"close"),
        "the additive one of the database: {unread:?}"
    );
    assert!(
        unread.contains(&"lift_add"),
        "the additive file: {unread:?}"
    );
    assert!(
        unread.contains(&"Data\\Objects\\broken.caf"),
        "the broken file: {unread:?}"
    );
    Ok(())
}

#[test]
fn a_source_that_does_not_exist_is_an_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(&dir)?;
    let index = PathIndex::new(&archive);

    let result = AnimationListing::from_archive(&index, MODEL, &["Data\\none.caf".to_owned()]);

    assert!(
        matches!(result, Err(ConversionError::UnknownEntry(_))),
        "{result:?}"
    );
    Ok(())
}
