// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Owned children in the Record tree: inline classes and strong pointers as nested structs.

use p4kx_dataforge::{Field, StructValue, Value};
use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestStruct, TestType, TestValue};

use crate::versions::{TestResult, in_both_versions};

const SAMPLE: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;

fn vector() -> TestStruct {
    TestStruct::new("Vector")
        .field("x", TestType::Single)
        .field("y", TestType::Single)
}

fn point(x: f32, y: f32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("Vector")
            .value("x", TestValue::Single(x))
            .value("y", TestValue::Single(y)),
    )
}

fn field<'t, 'f>(fields: &'t [Field<'f>], name: &str) -> Result<&'t Value<'f>, String> {
    fields
        .iter()
        .find(|field| field.name() == name)
        .map(Field::value)
        .ok_or_else(|| format!("no field {name}"))
}

fn as_struct<'t, 'f>(value: &'t Value<'f>) -> Result<&'t StructValue<'f>, String> {
    match value {
        Value::Struct(value) => Ok(value),
        other => Err(format!("not a struct: {other:?}")),
    }
}

#[test]
fn an_inline_class_is_a_nested_struct_between_its_neighbours() -> TestResult {
    let builder = DataForgeBuilder::new()
        .structure(vector())
        .structure(
            TestStruct::new("Ship")
                .field("crew", TestType::Int32)
                .field("position", TestType::Class("Vector".to_owned()))
                .field("mass", TestType::Single),
        )
        .record(
            TestRecord::new(SAMPLE, "Ship", "Ship.One", "ships/one.xml")
                .value("crew", TestValue::Int32(2))
                .value("position", point(1.5, -2.0))
                .value("mass", TestValue::Single(9.0)),
        );

    in_both_versions(&builder, |tree| {
        assert_eq!(field(tree.fields(), "crew")?, &Value::Int32(2), "before");
        assert_eq!(field(tree.fields(), "mass")?, &Value::Single(9.0), "after");
        let position = as_struct(field(tree.fields(), "position")?)?;
        assert_eq!(position.type_name(), "Vector", "the type of the value");
        assert_eq!(field(position.fields(), "x")?, &Value::Single(1.5), "x");
        assert_eq!(field(position.fields(), "y")?, &Value::Single(-2.0), "y");
        Ok(())
    })
}

#[test]
fn a_strong_pointer_is_a_nested_struct_of_the_type_it_points_to() -> TestResult {
    let builder = DataForgeBuilder::new()
        .structure(vector())
        .structure(
            TestStruct::new("Vector3")
                .parent("Vector")
                .field("z", TestType::Single),
        )
        .structure(
            TestStruct::new("Ship")
                .field("anchor", TestType::StrongPointer("Vector".to_owned()))
                .field("spare", TestType::StrongPointer("Vector".to_owned())),
        )
        .record(
            TestRecord::new(SAMPLE, "Ship", "Ship.One", "ships/one.xml").value(
                "anchor",
                TestValue::Struct(
                    TestInstance::new("Vector3")
                        .value("x", TestValue::Single(1.0))
                        .value("z", TestValue::Single(3.0)),
                ),
            ),
        );

    in_both_versions(&builder, |tree| {
        let anchor = as_struct(field(tree.fields(), "anchor")?)?;
        assert_eq!(anchor.type_name(), "Vector3", "the derived type");
        let names: Vec<_> = anchor.fields().iter().map(Field::name).collect();
        assert_eq!(names, ["x", "y", "z"], "inherited fields first");
        assert_eq!(field(anchor.fields(), "z")?, &Value::Single(3.0), "z");
        assert_eq!(
            field(tree.fields(), "spare")?,
            &Value::Null,
            "an explicit null"
        );
        Ok(())
    })
}

#[test]
fn owned_structs_nest_inside_each_other() -> TestResult {
    let builder = DataForgeBuilder::new()
        .structure(vector())
        .structure(
            TestStruct::new("Mount")
                .field("offset", TestType::Class("Vector".to_owned()))
                .field("tip", TestType::StrongPointer("Vector".to_owned())),
        )
        .structure(
            TestStruct::new("Ship").field("mount", TestType::StrongPointer("Mount".to_owned())),
        )
        .record(
            TestRecord::new(SAMPLE, "Ship", "Ship.One", "ships/one.xml").value(
                "mount",
                TestValue::Struct(
                    TestInstance::new("Mount")
                        .value("offset", point(0.5, 0.25))
                        .value("tip", point(7.0, 8.0)),
                ),
            ),
        );

    in_both_versions(&builder, |tree| {
        let mount = as_struct(field(tree.fields(), "mount")?)?;
        let offset = as_struct(field(mount.fields(), "offset")?)?;
        let tip = as_struct(field(mount.fields(), "tip")?)?;
        assert_eq!(field(offset.fields(), "y")?, &Value::Single(0.25), "inline");
        assert_eq!(field(tip.fields(), "x")?, &Value::Single(7.0), "pointer");
        Ok(())
    })
}

#[test]
fn fields_behind_nested_values_keep_their_offsets() -> TestResult {
    let builder = DataForgeBuilder::new()
        .structure(vector())
        .structure(
            TestStruct::new("Outer")
                .field("before", TestType::Int8)
                .field("inner", TestType::Class("Vector".to_owned()))
                .array("list", TestType::Int32)
                .field("owned", TestType::StrongPointer("Vector".to_owned()))
                .field("link", TestType::Reference)
                .field("after", TestType::Int8),
        )
        .record(
            TestRecord::new(SAMPLE, "Outer", "Outer.One", "a.xml")
                .value("before", TestValue::Int8(1))
                .value("after", TestValue::Int8(2)),
        );

    in_both_versions(&builder, |tree| {
        assert_eq!(field(tree.fields(), "before")?, &Value::Int8(1), "before");
        assert_eq!(field(tree.fields(), "after")?, &Value::Int8(2), "after");
        assert_eq!(
            field(tree.fields(), "list")?,
            &Value::Array(Box::new([])),
            "an empty array"
        );
        assert_eq!(
            field(tree.fields(), "owned")?,
            &Value::Null,
            "a null pointer"
        );
        assert_eq!(
            field(tree.fields(), "link")?,
            &Value::Null,
            "a null Reference"
        );
        assert_eq!(tree.errors().count(), 0, "no errors");
        Ok(())
    })
}
