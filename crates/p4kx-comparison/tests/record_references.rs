//! The field diff of the Record level for References, weak pointers and nested error values,
//! through the public API and the report.

mod forge_pair;
mod nested_diff;

use std::error::Error;

use forge_pair::ForgePair;
use nested_diff::{change, field_changes};
use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const ITEM: u128 = 0x2000_0000_0000_4000_8000_0000_0000_0001;
const MAKER: u128 = 0x2000_0000_0000_4000_8000_0000_0000_0002;
const OTHER_MAKER: u128 = 0x2000_0000_0000_4000_8000_0000_0000_0003;
const ITEM_TEXT: &str = "20000000-0000-4000-8000-000000000001";
const MAKER_TEXT: &str = "20000000-0000-4000-8000-000000000002";
const OTHER_MAKER_TEXT: &str = "20000000-0000-4000-8000-000000000003";

fn database(maker_path: &str, item: TestRecord) -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(TestStruct::new("Maker").field("name", TestType::Int32))
        .structure(TestStruct::new("Node").field("weight", TestType::Int32))
        .structure(
            TestStruct::new("Hull")
                .field("label", TestType::String)
                .field("armor", TestType::Int32),
        )
        .structure(
            TestStruct::new("Item")
                .field("maker", TestType::Reference)
                .array("nodes", TestType::Class("Node".to_owned()))
                .field("focus", TestType::WeakPointer("Node".to_owned()))
                .field("hull", TestType::Class("Hull".to_owned()))
                .field("speed", TestType::Int32),
        )
        .record(TestRecord::new(MAKER, "Maker", "Maker.One", maker_path))
        .record(TestRecord::new(
            OTHER_MAKER,
            "Maker",
            "Maker.Two",
            "libs/makers/two.xml",
        ))
        .record(item)
}

fn item(maker: TestValue, focus: &str) -> TestRecord {
    item_with(maker, focus, hull(TestValue::text("Hull"), 1), 1)
}

fn item_with(maker: TestValue, focus: &str, hull: TestValue, speed: i32) -> TestRecord {
    TestRecord::new(ITEM, "Item", "Item.Sample", "libs/items/sample.xml")
        .value("maker", maker)
        .value(
            "nodes",
            TestValue::Array(vec![
                TestValue::Struct(TestInstance::new("Node").named("first")),
                TestValue::Struct(TestInstance::new("Node").named("second")),
            ]),
        )
        .value("focus", TestValue::pointer(focus))
        .value("hull", hull)
        .value("speed", TestValue::Int32(speed))
}

fn reference(guid: &str, path: &str, at: &str) -> String {
    format!("ref {guid} {path} {at}")
}

#[test]
fn a_reference_to_another_record_shows_both_guids_and_record_paths() -> TestResult {
    let old = database(
        "libs/makers/one.xml",
        item(TestValue::Reference(MAKER), "first"),
    );
    let new = database(
        "libs/makers/one.xml",
        item(TestValue::Reference(OTHER_MAKER), "first"),
    );

    let changes = field_changes(&old, &new)?;

    assert_eq!(
        changes,
        [change(
            "maker",
            Some(&reference(MAKER_TEXT, "libs/makers/one", "-")),
            Some(&reference(OTHER_MAKER_TEXT, "libs/makers/two", "-")),
        )],
        "old and new target"
    );
    Ok(())
}

#[test]
fn a_reference_whose_target_moved_is_unchanged() -> TestResult {
    let old = database(
        "libs/makers/one.xml",
        item(TestValue::Reference(MAKER), "first"),
    );
    let new = database(
        "libs/makers/moved/one.xml",
        item(TestValue::Reference(MAKER), "first"),
    );

    let changes = field_changes(&old, &new)?;

    assert!(changes.is_empty(), "same target GUID: {changes:?}");
    Ok(())
}

#[test]
fn a_weak_pointer_to_another_place_in_the_record_is_a_change() -> TestResult {
    let old = database("libs/makers/one.xml", item(TestValue::Null, "first"));
    let new = database("libs/makers/one.xml", item(TestValue::Null, "second"));

    let changes = field_changes(&old, &new)?;

    assert_eq!(
        changes,
        [change(
            "focus",
            Some(&reference(ITEM_TEXT, "libs/items/sample", "nodes[0]")),
            Some(&reference(ITEM_TEXT, "libs/items/sample", "nodes[1]")),
        )],
        "same GUID, another location"
    );
    Ok(())
}

#[test]
fn a_null_reference_against_a_reference_is_a_change() -> TestResult {
    let null = database("libs/makers/one.xml", item(TestValue::Null, "first"));
    let set = database(
        "libs/makers/one.xml",
        item(TestValue::Reference(MAKER), "first"),
    );

    let changes = field_changes(&null, &set)?;

    assert_eq!(
        changes,
        [change(
            "maker",
            Some("Null"),
            Some(&reference(MAKER_TEXT, "libs/makers/one", "-")),
        )],
        "null to Reference"
    );
    Ok(())
}

fn hull(label: TestValue, armor: i32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("Hull")
            .value("label", label)
            .value("armor", TestValue::Int32(armor)),
    )
}

#[test]
fn a_nested_error_value_is_a_change_with_its_reason_and_the_record_goes_on() -> TestResult {
    let old = database("libs/makers/one.xml", item(TestValue::Null, "first"));
    let new = database(
        "libs/makers/one.xml",
        item_with(
            TestValue::Null,
            "first",
            hull(TestValue::Raw(u32::MAX.to_le_bytes().to_vec()), 2),
            2,
        ),
    );

    let changes = field_changes(&old, &new)?;

    assert_eq!(
        changes,
        [
            change(
                "hull.label",
                Some("String(\"Hull\")"),
                Some("Error(InvalidString { table: \"text\", offset: 4294967295 })"),
            ),
            change("hull.armor", Some("Int32(1)"), Some("Int32(2)")),
            change("speed", Some("Int32(1)"), Some("Int32(2)")),
        ],
        "the error at its place, the fields after it still compared"
    );
    Ok(())
}

#[test]
fn the_report_gives_references_and_nulls_with_their_type() -> TestResult {
    let old = database("libs/makers/one.xml", item(TestValue::Null, "first"));
    let new = database(
        "libs/makers/one.xml",
        item(TestValue::Reference(MAKER), "second"),
    );
    let pair = ForgePair::new(&old, &new)?;

    let fields = pair.compare_records(|comparison| {
        serde_json::to_value(comparison.report())
            .map(|report| report.pointer("/records/changes/0/fields").cloned())
    })??;

    assert_eq!(
        fields,
        Some(serde_json::json!([
            {
                "path": "maker",
                "old": {"type": "null"},
                "new": {"type": "reference", "value": {
                    "guid": MAKER_TEXT, "record_path": "libs/makers/one", "type": "Maker"
                }}
            },
            {
                "path": "focus",
                "old": {"type": "reference", "value": {
                    "guid": ITEM_TEXT, "record_path": "libs/items/sample", "type": "Node",
                    "at": "nodes[0]"
                }},
                "new": {"type": "reference", "value": {
                    "guid": ITEM_TEXT, "record_path": "libs/items/sample", "type": "Node",
                    "at": "nodes[1]"
                }}
            }
        ])),
        "References with GUID, record path, type and location"
    );
    Ok(())
}
