// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Record JSON of nested structs, arrays, References and nulls.

use std::error::Error;

use p4kx_dataforge::{DataForge, JsonLayout, RecordJson};
use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLE: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;
const MAKER: u128 = 0x50e8_a9f6_f956_4000_8ace_7c0c_a9a9_1f5d;
const MISSING: u128 = 0x0000_0001_0002_0003_0004_0000_0000_0005;

fn wheel(size: i32) -> TestValue {
    TestValue::Struct(TestInstance::new("Wheel").value("size", TestValue::Int32(size)))
}

fn car() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(
            TestStruct::new("Vector")
                .field("x", TestType::Single)
                .field("y", TestType::Single),
        )
        .structure(TestStruct::new("Wheel").field("size", TestType::Int32))
        .structure(TestStruct::new("Maker"))
        .structure(
            TestStruct::new("Car")
                .field("name", TestType::String)
                .field("position", TestType::Class("Vector".to_owned()))
                .array("wheels", TestType::Class("Wheel".to_owned()))
                .field("spare", TestType::StrongPointer("Wheel".to_owned()))
                .field("missing", TestType::StrongPointer("Wheel".to_owned()))
                .array("tags", TestType::String)
                .field("maker", TestType::Reference)
                .field("front", TestType::WeakPointer("Wheel".to_owned()))
                .array("parts", TestType::Reference)
                .field("lost", TestType::Reference),
        )
        .instance(
            "spare",
            TestInstance::new("Wheel").value("size", TestValue::Int32(15)),
        )
        .record(car_record())
        .record(TestRecord::new(
            MAKER,
            "Maker",
            "Maker.Aegis",
            "libs/foundry/records/makers/aegis.xml",
        ))
}

fn car_record() -> TestRecord {
    TestRecord::new(
        SAMPLE,
        "Car",
        "Car.One",
        "libs/foundry/records/cars/one.xml",
    )
    .value("name", TestValue::text("Avenger"))
    .value(
        "position",
        TestValue::Struct(
            TestInstance::new("Vector")
                .value("x", TestValue::Single(1.0))
                .value("y", TestValue::Single(2.0)),
        ),
    )
    .value("wheels", TestValue::Array(vec![wheel(16), wheel(17)]))
    .value("spare", TestValue::pointer("spare"))
    .value(
        "tags",
        TestValue::Array(vec![TestValue::text("a"), TestValue::text("b")]),
    )
    .value("maker", TestValue::Reference(MAKER))
    .value("front", TestValue::pointer("spare"))
    .value(
        "parts",
        TestValue::Array(vec![TestValue::Reference(MAKER), TestValue::Null]),
    )
    .value("lost", TestValue::Reference(MISSING))
}

/// Returns the JSON of the first Record, checking that versions 6 and 8 give the same bytes.
fn first_json(layout: JsonLayout) -> Result<String, Box<dyn Error>> {
    let mut texts = Vec::new();
    for version in [6, 8] {
        let bytes = car().version(version).build();
        let forge = DataForge::open(&bytes)?;
        let tree = forge.records().next().ok_or("no Record")?.tree()?;
        let mut out = Vec::new();
        RecordJson::new(layout).write(&tree, &mut out)?;
        texts.push(String::from_utf8(out)?);
    }
    let [six, eight] = <[String; 2]>::try_from(texts).map_err(|_| "two versions")?;
    assert_eq!(six, eight, "versions 6 and 8 give the same JSON");
    Ok(six)
}

#[test]
fn owned_children_are_nested_objects_and_references_are_links() -> TestResult {
    let json = first_json(JsonLayout::Pretty)?;

    assert_eq!(
        json,
        concat!(
            "{\n",
            "  \"$type\": \"Car\",\n",
            "  \"$guid\": \"cbee34fd-6441-4000-b99f-924f7f26e19f\",\n",
            "  \"$path\": \"libs/foundry/records/cars/one\",\n  \"$name\": \"Car.One\",\n",
            "  \"name\": \"Avenger\",\n",
            "  \"position\": {\n",
            "    \"$type\": \"Vector\",\n",
            "    \"x\": 1,\n",
            "    \"y\": 2\n",
            "  },\n",
            "  \"wheels\": [\n",
            "    {\n",
            "      \"$type\": \"Wheel\",\n",
            "      \"size\": 16\n",
            "    },\n",
            "    {\n",
            "      \"$type\": \"Wheel\",\n",
            "      \"size\": 17\n",
            "    }\n",
            "  ],\n",
            "  \"spare\": {\n",
            "    \"$type\": \"Wheel\",\n",
            "    \"size\": 15\n",
            "  },\n",
            "  \"missing\": null,\n",
            "  \"tags\": [\n",
            "    \"a\",\n",
            "    \"b\"\n",
            "  ],\n",
            "  \"maker\": {\n",
            "    \"guid\": \"50e8a9f6-f956-4000-8ace-7c0ca9a91f5d\",\n",
            "    \"path\": \"libs/foundry/records/makers/aegis\",\n",
            "    \"type\": \"Maker\"\n",
            "  },\n",
            "  \"front\": {\n",
            "    \"guid\": \"cbee34fd-6441-4000-b99f-924f7f26e19f\",\n",
            "    \"path\": \"libs/foundry/records/cars/one\",\n",
            "    \"type\": \"Wheel\",\n",
            "    \"at\": \"spare\"\n",
            "  },\n",
            "  \"parts\": [\n",
            "    {\n",
            "      \"guid\": \"50e8a9f6-f956-4000-8ace-7c0ca9a91f5d\",\n",
            "      \"path\": \"libs/foundry/records/makers/aegis\",\n",
            "      \"type\": \"Maker\"\n",
            "    },\n",
            "    null\n",
            "  ],\n",
            "  \"lost\": {\n",
            "    \"error\": \"the Reference refers to the Record 00000001-0002-0003-0004-000000000005, which does not exist\"\n",
            "  }\n",
            "}\n",
        ),
        "nested objects and arrays in the order of the Record XML"
    );
    Ok(())
}

#[test]
fn compact_and_pretty_layouts_hold_the_same_value() -> TestResult {
    let compact = first_json(JsonLayout::Compact)?;
    let pretty = first_json(JsonLayout::Pretty)?;

    assert!(!compact.contains('\n'), "one line: {compact}");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&compact)?,
        serde_json::from_str::<serde_json::Value>(&pretty)?,
        "the same value"
    );
    Ok(())
}
