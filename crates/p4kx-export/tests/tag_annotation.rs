//! The tag tree through the library API of `p4kx-export`: tag GUID to tag path, parent and
//! children, and the tag paths in exported Records.

use std::error::Error;
use std::fs;

use p4kx_export::{
    Guid, NDJSON_FILE, RecordExportOptions, RecordForm, TagResolver, TagTree, write_record_export,
};
use p4kx_extraction::DataForge;
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const DATABASE: u128 = 0x00d0_0000_0000_4000_8000_0000_0000_0001;
const WEAPON: u128 = 0x00a0_0000_0000_4000_8000_0000_0000_0001;
const GUN: u128 = 0x00a1_0000_0000_4000_8000_0000_0000_0001;
const ITEM: u128 = 0x0100_0000_0000_4000_8000_0000_0000_0001;
const PATH: &str = "libs/foundry/records/tagdatabase/tagdatabase.tagdatabase.xml";

fn references(guids: &[u128]) -> TestValue {
    TestValue::Array(guids.iter().map(|g| TestValue::Reference(*g)).collect())
}

fn tag(guid: u128, name: &str, children: &[u128]) -> TestRecord {
    TestRecord::new(guid, "Tag", format!("Tag.{name}"), PATH)
        .value("tagName", TestValue::text(name))
        .value("legacyGUID", TestValue::UInt32(u32::MAX))
        .value("children", references(children))
}

fn forge_bytes() -> Vec<u8> {
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
        .structure(
            TestStruct::new("Item").array("requiredTags", TestType::ReferenceTo("Tag".into())),
        )
        .record(
            TestRecord::new(DATABASE, "TagDatabase", "TagDatabase.TagDatabase", PATH)
                .value("tags", references(&[WEAPON])),
        )
        .record(tag(WEAPON, "Weapon", &[GUN]))
        .record(tag(GUN, "Gun", &[]))
        .record(
            TestRecord::new(ITEM, "Item", "Item.Sample", "libs/foundry/records/item.xml")
                .value("requiredTags", references(&[GUN])),
        )
        .build()
}

fn guid(value: u128) -> Result<Guid, Box<dyn Error>> {
    let text = format!("{value:032x}");
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
fn the_catalog_export_can_walk_the_tag_tree_without_the_cli_or_the_gui() -> TestResult {
    let bytes = forge_bytes();
    let forge = DataForge::open(&bytes)?;
    let tags = TagTree::build(&forge);

    let gun = tags.get(guid(GUN)?).ok_or("no Gun")?;
    assert_eq!(gun.path(), "Weapon.Gun", "tag GUID to tag path");
    assert_eq!(
        tags.tag_path(guid(GUN)?),
        Some("Weapon.Gun"),
        "as a resolver"
    );
    assert_eq!(
        gun.parent().map(|tag| tag.guid()),
        Some(guid(WEAPON)?),
        "parent"
    );
    let weapon = tags.get(guid(WEAPON)?).ok_or("no Weapon")?;
    let children: Vec<_> = weapon.children().map(|tag| tag.guid()).collect();
    assert_eq!(children, [guid(GUN)?], "children");
    assert!(
        gun.is_within(&weapon),
        "a paint matches a tag below the one a port requires"
    );
    Ok(())
}

#[test]
fn the_record_export_writes_the_tag_path_of_tag_references_when_asked() -> TestResult {
    let bytes = forge_bytes();
    let forge = DataForge::open(&bytes)?;
    let tags = TagTree::build(&forge);
    let records: Vec<_> = forge
        .records()
        .filter(|r| r.guid() == guid(ITEM).unwrap_or_default())
        .collect();
    assert_eq!(records.len(), 1, "the Item Record");
    let plain = tempfile::tempdir()?;
    let annotated = tempfile::tempdir()?;

    let summary = write_record_export(
        &forge,
        &records,
        RecordForm::Ndjson,
        plain.path(),
        &RecordExportOptions::default(),
    );
    assert!(summary.is_complete(), "{:?}", summary.failures().first());
    let options = RecordExportOptions {
        tags: Some(&tags),
        ..RecordExportOptions::default()
    };
    let summary = write_record_export(
        &forge,
        &records,
        RecordForm::Ndjson,
        annotated.path(),
        &options,
    );
    assert!(summary.is_complete(), "{:?}", summary.failures().first());

    let plain = fs::read_to_string(plain.path().join(NDJSON_FILE))?;
    let annotated = fs::read_to_string(annotated.path().join(NDJSON_FILE))?;
    assert!(
        !plain.contains("\"tag\""),
        "no tag path by default: {plain}"
    );
    assert!(
        annotated.contains("\"tag\":\"Weapon.Gun\""),
        "the tag path next to the Reference: {annotated}"
    );
    Ok(())
}
