// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Finding tags by tag path, and stopping the build of the tag tree.

use std::cell::Cell;
use std::error::Error;

use p4kx_dataforge::{DataForge, TagTree};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const DATABASE: u128 = 0x00d0_0000_0000_4000_8000_0000_0000_0001;
const PATH: &str = "libs/foundry/records/tagdatabase/tagdatabase.tagdatabase.xml";

const fn guid_of(index: usize) -> u128 {
    0x00a0_0000_0000_4000_8000_0000_0000_0000 + index as u128
}

fn references(guids: &[u128]) -> TestValue {
    TestValue::Array(guids.iter().map(|g| TestValue::Reference(*g)).collect())
}

fn tag(index: usize, name: &str, children: &[u128]) -> TestRecord {
    TestRecord::new(guid_of(index), "Tag", format!("Tag.{index}"), PATH)
        .value("tagName", TestValue::text(name))
        .value("legacyGUID", TestValue::UInt32(1))
        .value("children", references(children))
}

fn builder(roots: &[u128]) -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(
            TestStruct::new("Tag")
                .field("tagName", TestType::String)
                .field("legacyGUID", TestType::UInt32)
                .array("children", TestType::ReferenceTo("Tag".into())),
        )
        .structure(
            TestStruct::new("TagDatabase").array("tags", TestType::ReferenceTo("Tag".into())),
        )
        .record(
            TestRecord::new(DATABASE, "TagDatabase", "TagDatabase.TagDatabase", PATH)
                .value("tags", references(roots)),
        )
}

fn twins() -> Vec<u8> {
    builder(&[guid_of(0), guid_of(1)])
        .record(tag(0, "Weapon", &[guid_of(2)]))
        .record(tag(1, "Armour", &[guid_of(3)]))
        .record(tag(2, "Gun", &[]))
        .record(tag(3, "gun", &[]))
        .build()
}

fn flat(count: usize) -> Vec<u8> {
    let roots: Vec<u128> = (0..count).map(guid_of).collect();
    let mut builder = builder(&roots);
    for index in 0..count {
        builder = builder.record(tag(index, &format!("Tag{index}"), &[]));
    }
    builder.build()
}

#[test]
fn find_path_yields_every_tag_with_the_path_ignoring_case() -> TestResult {
    let bytes = builder(&[guid_of(0), guid_of(1)])
        .record(tag(0, "Weapon", &[guid_of(2)]))
        .record(tag(1, "weapon", &[]))
        .record(tag(2, "Gun", &[]))
        .build();
    let forge = DataForge::open(&bytes)?;
    let tags = TagTree::build(&forge);

    let found: Vec<&str> = tags.find_path("WEAPON").map(|tag| tag.path()).collect();

    assert_eq!(found, ["Weapon", "weapon"], "paths are not unique");
    assert_eq!(
        tags.find_path("Weapon.Gun").count(),
        1,
        "a path below a root"
    );
    assert_eq!(tags.find_path("Gun").count(), 0, "a name is not a path");
    assert_eq!(tags.find_path("").count(), 0, "no tag has an empty path");
    Ok(())
}

#[test]
fn find_path_ignores_case_in_every_part_of_the_path() -> TestResult {
    let bytes = twins();
    let forge = DataForge::open(&bytes)?;
    let tags = TagTree::build(&forge);

    let positions: Vec<usize> = tags
        .find_path("armour.gun")
        .map(|tag| tag.position())
        .collect();

    assert_eq!(positions, [3], "the one tag below Armour");
    Ok(())
}

#[test]
fn a_build_that_is_stopped_returns_nothing() -> TestResult {
    let bytes = flat(5000);
    let forge = DataForge::open(&bytes)?;

    assert!(
        TagTree::build_with(&forge, &|| true).is_none(),
        "stopped at once"
    );
    let polls = Cell::new(0_usize);
    let stopped = TagTree::build_with(&forge, &|| {
        polls.set(polls.get() + 1);
        polls.get() > 2
    });
    assert!(stopped.is_none(), "stopped after a few polls");
    assert!(polls.get() > 2, "polled while building");
    let built = TagTree::build_with(&forge, &|| false).ok_or("not built")?;
    assert_eq!(built.len(), 5000, "never stopped");
    assert_eq!(
        TagTree::build(&forge).len(),
        5000,
        "build is build_with that never stops"
    );
    Ok(())
}
