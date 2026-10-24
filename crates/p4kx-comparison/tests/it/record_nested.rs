// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The field diff of the Record level inside nested structs and arrays, through the public API.

use std::error::Error;

use crate::nested_diff::{change, field_changes};
use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const GUID: u128 = 0x2000_0000_0000_4000_8000_0000_0000_0001;

fn record() -> TestRecord {
    TestRecord::new(GUID, "Item", "Item.Sample", "libs/items/sample.xml")
}

fn schema() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(
            TestStruct::new("Plate")
                .field("armor", TestType::Int32)
                .field("tag", TestType::UInt8),
        )
        .structure(TestStruct::new("HeavyPlate").parent("Plate"))
        .structure(
            TestStruct::new("Hull")
                .field("plate", TestType::Class("Plate".to_owned()))
                .array("plates", TestType::Class("Plate".to_owned())),
        )
        .structure(
            TestStruct::new("Item")
                .field("hull", TestType::Class("Hull".to_owned()))
                .array("sizes", TestType::Int32)
                .field("spare", TestType::StrongPointer("Plate".to_owned())),
        )
}

fn plate(armor: i32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("Plate")
            .value("armor", TestValue::Int32(armor))
            .value("tag", TestValue::UInt8(1)),
    )
}

fn hull(plate_armor: i32, plates: Vec<TestValue>) -> TestValue {
    TestValue::Struct(
        TestInstance::new("Hull")
            .value("plate", plate(plate_armor))
            .value("plates", TestValue::Array(plates)),
    )
}

fn sizes(values: &[i32]) -> TestValue {
    TestValue::Array(values.iter().copied().map(TestValue::Int32).collect())
}

#[test]
fn a_change_inside_nested_structs_and_arrays_carries_the_full_path() -> TestResult {
    let old = schema().record(record().value("hull", hull(1, vec![plate(2), plate(3)])));
    let new = schema().record(record().value("hull", hull(4, vec![plate(2), plate(5)])));

    let changes = field_changes(&old, &new)?;

    assert_eq!(
        changes,
        [
            change("hull.plate.armor", Some("Int32(1)"), Some("Int32(4)")),
            change("hull.plates[1].armor", Some("Int32(3)"), Some("Int32(5)")),
        ],
        "struct names and array indices down to the value"
    );
    Ok(())
}

#[test]
fn equal_nested_values_are_unchanged() -> TestResult {
    let side = || {
        schema().record(
            record()
                .value("hull", hull(1, vec![plate(2)]))
                .value("sizes", sizes(&[1, 2]))
                .value("spare", plate(3)),
        )
    };

    let changes = field_changes(&side(), &side())?;

    assert!(changes.is_empty(), "no change: {changes:?}");
    Ok(())
}

#[test]
fn arrays_compare_by_index_with_added_elements_at_the_end() -> TestResult {
    let old = schema().record(record().value("sizes", sizes(&[1, 2])));
    let new = schema().record(record().value("sizes", sizes(&[1, 3, 4, 5])));

    let changes = field_changes(&old, &new)?;

    assert_eq!(
        changes,
        [
            change("sizes[1]", Some("Int32(2)"), Some("Int32(3)")),
            change("sizes[2]", None, Some("Int32(4)")),
            change("sizes[3]", None, Some("Int32(5)")),
        ],
        "element 1 changed, elements 2 and 3 added"
    );
    Ok(())
}

#[test]
fn removed_struct_elements_are_reported_with_their_fields() -> TestResult {
    let old = schema().record(record().value("hull", hull(1, vec![plate(2), plate(3)])));
    let new = schema().record(record().value("hull", hull(1, vec![plate(2)])));

    let changes = field_changes(&old, &new)?;

    assert_eq!(
        changes,
        [
            change("hull.plates[1]", Some("struct Plate"), None),
            change("hull.plates[1].armor", Some("Int32(3)"), None),
            change("hull.plates[1].tag", Some("UInt8(1)"), None),
        ],
        "the element itself, then every field it held"
    );
    Ok(())
}

#[test]
fn null_against_a_struct_and_back_is_a_change() -> TestResult {
    let null = schema().record(record().value("spare", TestValue::Null));
    let set = schema().record(record().value("spare", plate(7)));

    let filled = field_changes(&null, &set)?;
    let cleared = field_changes(&set, &null)?;

    assert_eq!(
        filled,
        [
            change("spare", Some("Null"), Some("struct Plate")),
            change("spare.armor", None, Some("Int32(7)")),
            change("spare.tag", None, Some("UInt8(1)")),
        ],
        "null to value"
    );
    assert_eq!(
        cleared,
        [
            change("spare", Some("struct Plate"), Some("Null")),
            change("spare.armor", Some("Int32(7)"), None),
            change("spare.tag", Some("UInt8(1)"), None),
        ],
        "value to null"
    );
    Ok(())
}

#[test]
fn a_struct_of_another_type_is_a_change_and_its_fields_are_compared() -> TestResult {
    let old = schema().record(record().value("spare", plate(7)));
    let new = schema().record(
        record().value(
            "spare",
            TestValue::Struct(
                TestInstance::new("HeavyPlate")
                    .value("armor", TestValue::Int32(9))
                    .value("tag", TestValue::UInt8(1)),
            ),
        ),
    );

    let changes = field_changes(&old, &new)?;

    assert_eq!(
        changes,
        [
            change("spare", Some("struct Plate"), Some("struct HeavyPlate")),
            change("spare.armor", Some("Int32(7)"), Some("Int32(9)")),
        ],
        "the type, then the fields joined by name"
    );
    Ok(())
}

#[test]
fn a_field_that_changes_between_scalar_and_nested_is_a_change() -> TestResult {
    let scalar = DataForgeBuilder::new()
        .structure(TestStruct::new("Item").field("hull", TestType::Int32))
        .record(record().value("hull", TestValue::Int32(3)));
    let nested = schema().record(record().value("hull", hull(1, vec![plate(2)])));

    let to_nested = field_changes(&scalar, &nested)?;
    let to_scalar = field_changes(&nested, &scalar)?;

    let nested_side = [
        "hull.plate",
        "hull.plate.armor",
        "hull.plate.tag",
        "hull.plates",
        "hull.plates[0]",
        "hull.plates[0].armor",
        "hull.plates[0].tag",
    ];
    assert_eq!(
        to_nested.first(),
        Some(&change("hull", Some("Int32(3)"), Some("struct Hull"))),
        "the field itself: {to_nested:?}"
    );
    assert_eq!(
        to_nested
            .iter()
            .skip(1)
            .filter(|(_, old, new)| old.is_none() && new.is_some())
            .map(|(path, ..)| path.as_str())
            .take(nested_side.len())
            .collect::<Vec<_>>(),
        nested_side,
        "every nested value added: {to_nested:?}"
    );
    assert_eq!(
        to_scalar.first(),
        Some(&change("hull", Some("struct Hull"), Some("Int32(3)"))),
        "back to the scalar: {to_scalar:?}"
    );
    assert!(
        to_scalar
            .iter()
            .skip(1)
            .take(nested_side.len())
            .all(|(_, old, new)| old.is_some() && new.is_none()),
        "every nested value removed: {to_scalar:?}"
    );
    Ok(())
}

#[test]
fn a_field_that_changes_between_scalar_and_array_is_a_change() -> TestResult {
    let scalar = DataForgeBuilder::new()
        .structure(TestStruct::new("Item").field("sizes", TestType::Int32))
        .record(record().value("sizes", TestValue::Int32(3)));
    let array = schema().record(record().value("sizes", sizes(&[3])));

    let changes = field_changes(&scalar, &array)?;

    assert_eq!(
        changes
            .iter()
            .filter(|(path, ..)| path.starts_with("sizes"))
            .cloned()
            .collect::<Vec<_>>(),
        [
            change("sizes", Some("Int32(3)"), Some("array 1")),
            change("sizes[0]", None, Some("Int32(3)")),
        ],
        "the array by its length, then its elements"
    );
    Ok(())
}
