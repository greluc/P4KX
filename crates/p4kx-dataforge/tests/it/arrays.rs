// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Arrays in the Record tree: scalars, structs, pointers and References, in stored order.

use p4kx_dataforge::{Field, Guid, RecordTree, Value};
use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestStruct, TestType, TestValue};

use crate::versions::{TestResult, in_both_versions};

const SAMPLE: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;
const OTHER: u128 = 0x50e8_a9f6_f956_4000_8ace_7c0c_a9a9_1f5d;

fn items<'t, 'f>(tree: &'t RecordTree<'f>, name: &str) -> Result<&'t [Value<'f>], String> {
    match tree
        .fields()
        .iter()
        .find(|field| field.name() == name)
        .map(Field::value)
    {
        Some(Value::Array(items)) => Ok(items),
        other => Err(format!("{name} is no array: {other:?}")),
    }
}

fn wheel(size: i32) -> TestValue {
    TestValue::Struct(TestInstance::new("Wheel").value("size", TestValue::Int32(size)))
}

#[test]
fn arrays_of_scalars_hold_their_values_in_order() -> TestResult {
    let builder = DataForgeBuilder::new()
        .enumeration("Size", ["Small", "Large"])
        .structure(
            TestStruct::new("Lists")
                .array("numbers", TestType::Int32)
                .array("flags", TestType::Bool)
                .array("names", TestType::String)
                .array("sizes", TestType::Enum("Size".to_owned()))
                .array("ids", TestType::Guid)
                .array("ratios", TestType::Double),
        )
        .record(
            TestRecord::new(SAMPLE, "Lists", "Lists.One", "lists.xml")
                .value(
                    "numbers",
                    TestValue::Array(vec![TestValue::Int32(3), TestValue::Int32(-1)]),
                )
                .value(
                    "flags",
                    TestValue::Array(vec![TestValue::Bool(false), TestValue::Bool(true)]),
                )
                .value(
                    "names",
                    TestValue::Array(vec![TestValue::text("b"), TestValue::text("a")]),
                )
                .value("sizes", TestValue::Array(vec![TestValue::text("Large")]))
                .value("ids", TestValue::Array(vec![TestValue::Guid(OTHER)]))
                .value("ratios", TestValue::Array(vec![TestValue::Double(0.5)])),
        );

    in_both_versions(&builder, |tree| {
        assert_eq!(
            items(tree, "numbers")?,
            [Value::Int32(3), Value::Int32(-1)],
            "int32"
        );
        assert_eq!(
            items(tree, "flags")?,
            [Value::Bool(false), Value::Bool(true)],
            "bool"
        );
        assert_eq!(
            items(tree, "names")?,
            [Value::String("b"), Value::String("a")],
            "string"
        );
        assert_eq!(items(tree, "sizes")?, [Value::Enum("Large")], "enum");
        assert_eq!(
            items(tree, "ids")?,
            [Value::Guid(
                "50e8a9f6-f956-4000-8ace-7c0ca9a91f5d".parse::<Guid>()?
            )],
            "GUID"
        );
        assert_eq!(items(tree, "ratios")?, [Value::Double(0.5)], "double");
        Ok(())
    })
}

#[test]
fn arrays_of_classes_and_strong_pointers_hold_structs_and_nulls() -> TestResult {
    let builder = DataForgeBuilder::new()
        .structure(TestStruct::new("Wheel").field("size", TestType::Int32))
        .structure(
            TestStruct::new("Car")
                .array("wheels", TestType::Class("Wheel".to_owned()))
                .array("spares", TestType::StrongPointer("Wheel".to_owned())),
        )
        .record(
            TestRecord::new(SAMPLE, "Car", "Car.One", "car.xml")
                .value("wheels", TestValue::Array(vec![wheel(16), wheel(17)]))
                .value("spares", TestValue::Array(vec![TestValue::Null, wheel(18)])),
        );

    in_both_versions(&builder, |tree| {
        let sizes = |values: &[Value<'_>]| -> Vec<String> {
            values
                .iter()
                .map(|value| match value {
                    Value::Struct(wheel) => format!(
                        "{}:{:?}",
                        wheel.type_name(),
                        wheel.fields().first().map(Field::value)
                    ),
                    other => format!("{other:?}"),
                })
                .collect()
        };
        assert_eq!(
            sizes(items(tree, "wheels")?),
            ["Wheel:Some(Int32(16))", "Wheel:Some(Int32(17))"],
            "class array"
        );
        assert_eq!(
            sizes(items(tree, "spares")?),
            ["Null", "Wheel:Some(Int32(18))"],
            "strong pointer array"
        );
        Ok(())
    })
}

#[test]
fn arrays_of_references_hold_links_and_nulls() -> TestResult {
    let builder = DataForgeBuilder::new()
        .structure(TestStruct::new("Box").array("links", TestType::Reference))
        .record(
            TestRecord::new(SAMPLE, "Box", "Box.One", "boxes/one.xml").value(
                "links",
                TestValue::Array(vec![TestValue::Reference(OTHER), TestValue::Null]),
            ),
        )
        .record(TestRecord::new(OTHER, "Box", "Box.Two", "boxes/two.xml"));

    in_both_versions(&builder, |tree| {
        let links = items(tree, "links")?;
        let [Value::Reference(link), Value::Null] = links else {
            return Err(format!("a Reference and a null: {links:?}").into());
        };
        assert_eq!(
            link.guid().to_string(),
            "50e8a9f6-f956-4000-8ace-7c0ca9a91f5d",
            "GUID"
        );
        assert_eq!(link.record_path(), "boxes/two", "record path");
        Ok(())
    })
}
