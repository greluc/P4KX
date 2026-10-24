// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The field diff of the Record level for scalar edge cases, through the public API.

use std::error::Error;

use crate::forge_pair::ForgePair;
use crate::nested_diff::{change, field_changes};
use p4kx_comparison::{ChangedValue, RecordChange, Value, ValueError};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const GUID: u128 = 0x2000_0000_0000_4000_8000_0000_0000_0001;

fn database(structure: TestStruct, values: &[(&str, TestValue)]) -> DataForgeBuilder {
    let record = values.iter().fold(
        TestRecord::new(GUID, "Item", "Item.Sample", "libs/items/sample.xml"),
        |record, (name, value)| record.value(*name, value.clone()),
    );
    DataForgeBuilder::new()
        .enumeration("Size", ["Small", "Medium", "Large"])
        .structure(structure)
        .record(record)
}

fn floats() -> TestStruct {
    TestStruct::new("Item")
        .field("single", TestType::Single)
        .field("double", TestType::Double)
}

#[test]
fn floats_are_compared_by_their_bits() -> TestResult {
    let old = database(
        floats(),
        &[
            ("single", TestValue::Single(f32::NAN)),
            ("double", TestValue::Double(0.0)),
        ],
    );
    let new = database(
        floats(),
        &[
            ("single", TestValue::Single(f32::from_bits(0x7fc0_0001))),
            ("double", TestValue::Double(-0.0)),
        ],
    );

    let changes = field_changes(&old, &new)?;

    assert_eq!(
        changes,
        [change("double", Some("Double(0.0)"), Some("Double(-0.0)"))],
        "every NaN equals every NaN, -0.0 differs from 0.0"
    );
    Ok(())
}

#[test]
fn equal_floats_are_unchanged() -> TestResult {
    let values = [
        ("single", TestValue::Single(0.1)),
        ("double", TestValue::Double(f64::INFINITY)),
    ];

    let changes = field_changes(&database(floats(), &values), &database(floats(), &values))?;

    assert!(changes.is_empty(), "no change: {changes:?}");
    Ok(())
}

#[test]
fn an_enum_change_shows_both_option_names() -> TestResult {
    let item = || TestStruct::new("Item").field("size", TestType::Enum("Size".to_owned()));

    let changes = field_changes(
        &database(item(), &[("size", TestValue::text("Small"))]),
        &database(item(), &[("size", TestValue::text("Large"))]),
    )?;

    assert_eq!(
        changes,
        [change(
            "size",
            Some("Enum(\"Small\")"),
            Some("Enum(\"Large\")")
        )],
        "both names"
    );
    Ok(())
}

#[test]
fn an_error_value_on_one_side_is_a_change_with_the_reason() -> TestResult {
    let item = || {
        TestStruct::new("Item")
            .field("label", TestType::String)
            .field("flag", TestType::Bool)
    };
    let invalid_offset = TestValue::Raw(u32::MAX.to_le_bytes().to_vec());

    let pair = ForgePair::new(
        &database(
            item(),
            &[
                ("label", TestValue::text("Hello")),
                ("flag", TestValue::Bool(true)),
            ],
        ),
        &database(
            item(),
            &[("label", invalid_offset), ("flag", TestValue::Raw(vec![7]))],
        ),
    )?;
    let reasons = pair.compare_records(|comparison| {
        comparison
            .record_changes()
            .iter()
            .flat_map(RecordChange::fields)
            .map(|field| match field.new_value() {
                Some(ChangedValue::Value(Value::Error(error))) => {
                    (field.path().to_string(), Some(*error), error.to_string())
                }
                _ => (field.path().to_string(), None, String::new()),
            })
            .collect::<Vec<_>>()
    })?;

    assert_eq!(
        reasons,
        [
            (
                "label".to_owned(),
                Some(ValueError::InvalidString {
                    table: "text",
                    offset: u32::MAX
                }),
                "offset 4294967295 in the text table is not a valid string".to_owned()
            ),
            (
                "flag".to_owned(),
                Some(ValueError::InvalidBool { byte: 7 }),
                "the bool byte 7 is neither 0 nor 1".to_owned()
            ),
        ],
        "both errors with their reason"
    );
    Ok(())
}

#[test]
fn a_field_only_one_side_has_is_a_change_with_one_value() -> TestResult {
    let old = database(
        TestStruct::new("Item")
            .field("speed", TestType::Int32)
            .field("legacy", TestType::UInt8),
        &[
            ("speed", TestValue::Int32(1)),
            ("legacy", TestValue::UInt8(9)),
        ],
    );
    let new = database(
        TestStruct::new("Item")
            .field("armor", TestType::Int16)
            .field("speed", TestType::Int32),
        &[
            ("armor", TestValue::Int16(5)),
            ("speed", TestValue::Int32(2)),
        ],
    );

    let changes = field_changes(&old, &new)?;

    assert_eq!(
        changes,
        [
            change("armor", None, Some("Int16(5)")),
            change("speed", Some("Int32(1)"), Some("Int32(2)")),
            change("legacy", Some("UInt8(9)"), None),
        ],
        "new field order first, removed fields last"
    );
    Ok(())
}

#[test]
fn a_field_whose_type_changed_is_a_change() -> TestResult {
    let changes = field_changes(
        &database(
            TestStruct::new("Item").field("value", TestType::Int32),
            &[("value", TestValue::Int32(0))],
        ),
        &database(
            TestStruct::new("Item").field("value", TestType::Single),
            &[("value", TestValue::Single(0.0))],
        ),
    )?;

    assert_eq!(
        changes,
        [change("value", Some("Int32(0)"), Some("Single(0.0)"))],
        "a different kind of value"
    );
    Ok(())
}

#[test]
fn every_scalar_kind_reports_a_change() -> TestResult {
    let item = || {
        TestStruct::new("Item")
            .field("i8", TestType::Int8)
            .field("u64", TestType::UInt64)
            .field("text", TestType::String)
            .field("label", TestType::Locale)
            .field("id", TestType::Guid)
    };

    let changes = field_changes(
        &database(
            item(),
            &[
                ("i8", TestValue::Int8(-1)),
                ("u64", TestValue::UInt64(1)),
                ("text", TestValue::text("a")),
                ("label", TestValue::text("@a")),
                ("id", TestValue::Guid(1)),
            ],
        ),
        &database(
            item(),
            &[
                ("i8", TestValue::Int8(1)),
                ("u64", TestValue::UInt64(u64::MAX)),
                ("text", TestValue::text("b")),
                ("label", TestValue::text("@b")),
                ("id", TestValue::Guid(2)),
            ],
        ),
    )?;

    assert_eq!(
        changes
            .iter()
            .map(|(path, ..)| path.as_str())
            .collect::<Vec<_>>(),
        ["i8", "u64", "text", "label", "id"],
        "every field changed: {changes:?}"
    );
    Ok(())
}
