// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The limits of the tag tree: depth, name and path length and duplicate GUIDs, over synthetic
//! DataForges built to be hostile.

use std::error::Error;

use p4kx_dataforge::{DataForge, MAX_TAG_DEPTH, MAX_TAG_NAME_BYTES, MAX_TAG_PATH_BYTES, TagTree};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const DATABASE: u128 = 0x0d00_0000_0000_4000_8000_0000_0000_0001;
const PATH: &str = "libs/foundry/records/tagdatabase/tagdatabase.tagdatabase.xml";

const fn guid_of(index: usize) -> u128 {
    0x00a0_0000_0000_4000_8000_0000_0000_0000 + index as u128
}

fn references(guids: &[u128]) -> TestValue {
    TestValue::Array(guids.iter().map(|g| TestValue::Reference(*g)).collect())
}

fn tag(guid: u128, name: &str, children: &[u128]) -> TestRecord {
    TestRecord::new(guid, "Tag", format!("Tag.{guid:x}"), PATH)
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

/// Returns a DataForge with a chain of `length` tags, each the only child of the one before, and
/// the name of tag `index` from `name`.
fn chain(length: usize, name: impl Fn(usize) -> String) -> Vec<u8> {
    let mut builder = builder(&[guid_of(0)]);
    for index in 0..length {
        let children = if index + 1 < length {
            vec![guid_of(index + 1)]
        } else {
            Vec::new()
        };
        builder = builder.record(tag(guid_of(index), &name(index), &children));
    }
    builder.build()
}

#[test]
fn a_chain_deeper_than_the_limit_is_cut_and_not_rooted_again() -> TestResult {
    let bytes = chain(MAX_TAG_DEPTH + 6, |index| format!("T{index}"));
    let forge = DataForge::open(&bytes)?;

    let tags = TagTree::build(&forge);

    assert_eq!(tags.len(), MAX_TAG_DEPTH, "the first 64 levels");
    let issues = tags.issues();
    assert_eq!(issues.too_deep(), 1, "the one child at the limit, once");
    assert_eq!(
        issues.unlisted(),
        0,
        "the tags below it are not roots of their own"
    );
    assert_eq!(tags.roots().count(), 1, "one root");
    let deepest = tags.iter().last().ok_or("no tag")?;
    assert_eq!(deepest.depth(), MAX_TAG_DEPTH - 1, "the deepest kept tag");
    assert!(
        tags.get(forge_guid(MAX_TAG_DEPTH)?).is_none(),
        "the first tag left out has no path"
    );
    Ok(())
}

fn forge_guid(index: usize) -> Result<p4kx_dataforge::Guid, Box<dyn Error>> {
    let text = format!("{:032x}", guid_of(index));
    let dashed = format!(
        "{}-{}-{}-{}-{}",
        &text[0..8],
        &text[8..12],
        &text[12..16],
        &text[16..20],
        &text[20..32]
    );
    Ok(dashed.parse()?)
}

#[test]
fn huge_names_in_a_chain_keep_the_tree_small() -> TestResult {
    let bytes = chain(40, |_| "x".repeat(64 << 10));
    let forge = DataForge::open(&bytes)?;

    let tags = TagTree::build(&forge);

    assert!(tags.is_empty(), "no name of 64 KiB is kept");
    assert_eq!(
        tags.issues().too_long(),
        1,
        "the root, with everything below it"
    );
    assert_eq!(
        tags.issues().unlisted(),
        0,
        "nothing below it is rooted instead"
    );
    Ok(())
}

#[test]
fn a_path_over_the_limit_cuts_the_chain_there() -> TestResult {
    let name = "n".repeat(MAX_TAG_NAME_BYTES);
    let bytes = chain(20, |_| name.clone());
    let forge = DataForge::open(&bytes)?;

    let tags = TagTree::build(&forge);

    let kept = MAX_TAG_PATH_BYTES / (MAX_TAG_NAME_BYTES + 1);
    assert_eq!(tags.len(), kept, "as many levels as fit");
    assert!(
        tags.iter()
            .all(|tag| tag.path().len() <= MAX_TAG_PATH_BYTES),
        "every path is within the limit"
    );
    assert_eq!(
        tags.issues().too_long(),
        1,
        "the first tag that does not fit"
    );
    assert_eq!(
        tags.issues().unlisted(),
        0,
        "and nothing below it is rooted instead"
    );
    let last = tags.iter().last().ok_or("no tag")?;
    assert_eq!(last.name(), name, "the name is the end of the path");
    Ok(())
}

#[test]
fn a_name_at_the_limit_is_kept_and_a_longer_one_is_not() -> TestResult {
    let bytes = builder(&[guid_of(0), guid_of(1)])
        .record(tag(guid_of(0), &"a".repeat(MAX_TAG_NAME_BYTES), &[]))
        .record(tag(guid_of(1), &"b".repeat(MAX_TAG_NAME_BYTES + 1), &[]))
        .build();
    let forge = DataForge::open(&bytes)?;

    let tags = TagTree::build(&forge);

    assert_eq!(tags.len(), 1, "only the first");
    assert_eq!(tags.issues().too_long(), 1);
    Ok(())
}

#[test]
fn a_repeated_guid_keeps_the_first_tag_and_is_counted() -> TestResult {
    let bytes = builder(&[guid_of(0)])
        .record(tag(guid_of(0), "First", &[]))
        .record(tag(guid_of(0), "Second", &[]))
        .build();
    let forge = DataForge::open(&bytes)?;

    let tags = TagTree::build(&forge);

    assert_eq!(tags.len(), 1, "one tag for the GUID");
    assert_eq!(tags.issues().duplicate(), 1, "the other is counted");
    assert_eq!(
        tags.path_of(forge_guid(0)?),
        Some("First"),
        "the first in definition order"
    );
    assert!(!tags.issues().is_clean());
    Ok(())
}
