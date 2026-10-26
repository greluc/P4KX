// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The paths of field statistics: the paths one level below a path, the Query path of a field
//! path and the text of a number.

use p4kx_dataforge::{DataForge, FieldStatistics, Number};
use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestStruct, TestType, TestValue};

use crate::field_common::{TestResult, database, paths};

fn nested() -> Vec<u8> {
    DataForgeBuilder::new()
        .structure(TestStruct::new("Vector").field("x", TestType::Single))
        .structure(TestStruct::new("Params"))
        .structure(
            TestStruct::new("WeaponParams")
                .parent("Params")
                .field("fireRate", TestType::Single),
        )
        .structure(
            TestStruct::new("Holder")
                .field("single", TestType::StrongPointer("Params".to_owned()))
                .array("many", TestType::StrongPointer("Params".to_owned()))
                .array("items", TestType::Class("Vector".to_owned()))
                .array("numbers", TestType::Int32),
        )
        .record(
            TestRecord::new(1, "Holder", "Holder.One", "holders/one.xml")
                .value(
                    "single",
                    TestValue::Struct(
                        TestInstance::new("WeaponParams").value("fireRate", TestValue::Single(2.0)),
                    ),
                )
                .value(
                    "many",
                    TestValue::Array(vec![TestValue::Struct(
                        TestInstance::new("WeaponParams").value("fireRate", TestValue::Single(3.0)),
                    )]),
                )
                .value(
                    "items",
                    TestValue::Array(vec![TestValue::Struct(
                        TestInstance::new("Vector").value("x", TestValue::Single(1.0)),
                    )]),
                )
                .value("numbers", TestValue::Array(vec![TestValue::Int32(4)])),
        )
        .build()
}

#[test]
fn children_are_the_paths_one_level_below_a_path() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;
    let statistics = FieldStatistics::of_all(&forge)?;
    let children = |parent: &str| -> Vec<String> {
        statistics
            .children(parent)
            .map(|field| field.path().to_owned())
            .collect()
    };

    assert_eq!(
        children("Components"),
        ["Components/Params", "Components/WeaponParams"],
        "the structs a pointer array owns"
    );
    assert_eq!(
        children("Components/WeaponParams"),
        ["Components/WeaponParams/fireRate"],
        "a field of an owned struct"
    );
    assert_eq!(children("position"), ["position/x", "position/y"], "inline");
    assert!(
        children("Comp").is_empty(),
        "a part of a segment is no path"
    );
    assert!(children("missing").is_empty(), "an unknown path");
    assert!(children("name").is_empty(), "a scalar has no fields");
    let top = children("");
    assert!(
        top.iter().all(|path| !path.contains('/')),
        "the Record's own fields: {top:?}"
    );
    assert_eq!(
        top.len(),
        paths(&statistics)
            .iter()
            .filter(|path| !path.contains('/'))
            .count(),
        "all of them"
    );
    Ok(())
}

#[test]
fn a_field_path_is_written_as_a_query_path() -> TestResult {
    let bytes = nested();
    let forge = DataForge::open(&bytes)?;
    let statistics = FieldStatistics::of_all(&forge)?;

    let query = |path: &str| statistics.query_path(path);

    assert_eq!(query("many"), Some("many".to_owned()), "a pointer array");
    assert_eq!(
        query("many/WeaponParams/fireRate"),
        Some("many[WeaponParams].fireRate".to_owned()),
        "the owned struct of a pointer array becomes a bracket"
    );
    assert_eq!(
        query("single/WeaponParams/fireRate"),
        Some("single.fireRate".to_owned()),
        "a single pointer needs no bracket"
    );
    assert_eq!(
        query("items/x"),
        Some("items[*].x".to_owned()),
        "the fields of inline structs in an array"
    );
    assert_eq!(
        query("numbers"),
        Some("numbers[*]".to_owned()),
        "an array of scalars"
    );
    assert_eq!(query("missing"), None, "an unknown path");
    assert_eq!(query("many/Missing/fireRate"), None, "an unknown segment");
    Ok(())
}

#[test]
fn a_nested_field_path_of_the_ship_database_is_a_query_path() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;
    let statistics = FieldStatistics::of_all(&forge)?;

    assert_eq!(
        statistics.query_path("Components/WeaponParams/fireRate"),
        Some("Components[WeaponParams].fireRate".to_owned())
    );
    assert_eq!(
        statistics.query_path("position/x"),
        Some("position.x".to_owned())
    );
    Ok(())
}

#[test]
fn very_large_and_very_small_floats_print_with_an_exponent() {
    let text = |number: Number| number.to_string();

    assert_eq!(text(Number::Single(1e30)), "1e30");
    assert_eq!(text(Number::Double(1.5e-7)), "1.5e-7");
    assert_eq!(text(Number::Single(2.5)), "2.5");
    assert_eq!(text(Number::Double(0.0)), "0");
    assert_eq!(text(Number::Single(123_456.0)), "123456");
    assert_eq!(text(Number::Double(-1e20)), "-1e20");
    assert_eq!(
        text(Number::Integer(10_i128.pow(30))),
        "1000000000000000000000000000000"
    );
}
