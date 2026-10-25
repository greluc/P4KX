// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Writing the canonical Record tree as Record JSON.

use std::error::Error;

use p4kx_dataforge::{DataForge, JsonLayout, RecordJson};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const GUID: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;

fn json_of(builder: &DataForgeBuilder, layout: JsonLayout) -> Result<String, Box<dyn Error>> {
    let bytes = builder.build();
    let forge = DataForge::open(&bytes)?;
    let tree = forge.records().next().ok_or("no Record")?.tree()?;
    let mut out = Vec::new();
    RecordJson::new(layout).write(&tree, &mut out)?;
    Ok(String::from_utf8(out)?)
}

fn one_record(structure: TestStruct, record: TestRecord) -> DataForgeBuilder {
    DataForgeBuilder::new()
        .enumeration("Size", ["Small", "Large"])
        .structure(structure)
        .record(record)
}

fn record(type_name: &str) -> TestRecord {
    TestRecord::new(
        GUID,
        type_name,
        format!("{type_name}.Sample"),
        "libs/foundry/records/sample.xml",
    )
}

fn scalars() -> DataForgeBuilder {
    one_record(
        TestStruct::new("Scalars")
            .field("flag", TestType::Bool)
            .field("i8", TestType::Int8)
            .field("i16", TestType::Int16)
            .field("i32", TestType::Int32)
            .field("i64", TestType::Int64)
            .field("u8", TestType::UInt8)
            .field("u16", TestType::UInt16)
            .field("u32", TestType::UInt32)
            .field("u64", TestType::UInt64)
            .field("single", TestType::Single)
            .field("double", TestType::Double)
            .field("text", TestType::String)
            .field("label", TestType::Locale)
            .field("id", TestType::Guid)
            .field("size", TestType::Enum("Size".to_owned())),
        record("Scalars")
            .value("flag", TestValue::Bool(true))
            .value("i8", TestValue::Int8(-8))
            .value("i16", TestValue::Int16(-1600))
            .value("i32", TestValue::Int32(-320_000))
            .value("i64", TestValue::Int64(i64::MIN))
            .value("u8", TestValue::UInt8(200))
            .value("u16", TestValue::UInt16(65_000))
            .value("u32", TestValue::UInt32(4_000_000_000))
            .value("u64", TestValue::UInt64(u64::MAX))
            .value("single", TestValue::Single(1.5))
            .value("double", TestValue::Double(-0.25))
            .value("text", TestValue::text("Hello \"World\" \\ \u{1}\u{8}\t\n"))
            .value("label", TestValue::text("@item_Name_Sample"))
            .value(
                "id",
                TestValue::Guid(0x50e8_a9f6_f956_4000_8ace_7c0c_a9a9_1f5d),
            )
            .value("size", TestValue::text("Large")),
    )
}

#[test]
fn a_compact_record_is_one_line_with_type_guid_path_and_every_scalar() -> TestResult {
    let json = json_of(&scalars(), JsonLayout::Compact)?;

    assert_eq!(
        json,
        concat!(
            "{\"$type\":\"Scalars\",\"$guid\":\"cbee34fd-6441-4000-b99f-924f7f26e19f\",",
            "\"$path\":\"libs/foundry/records/sample\",\"$name\":\"Scalars.Sample\",\"flag\":true,\"i8\":-8,\"i16\":-1600,",
            "\"i32\":-320000,\"i64\":-9223372036854775808,\"u8\":200,\"u16\":65000,",
            "\"u32\":4000000000,\"u64\":18446744073709551615,\"single\":1.5,\"double\":-0.25,",
            "\"text\":\"Hello \\\"World\\\" \\\\ \\u0001\\u0008\\t\\n\",\"label\":\"@item_Name_Sample\",",
            "\"id\":\"50e8a9f6-f956-4000-8ace-7c0ca9a91f5d\",\"size\":\"Large\"}",
        ),
        "one object without whitespace or line ending"
    );
    serde_json::from_str::<serde_json::Value>(&json)?;
    Ok(())
}

#[test]
fn a_pretty_record_is_indented_by_two_spaces_and_ends_with_a_line_ending() -> TestResult {
    let builder = one_record(
        TestStruct::new("Item")
            .field("mass", TestType::Single)
            .array("tags", TestType::String)
            .array("none", TestType::Int8),
        record("Item")
            .value("mass", TestValue::Single(2.5))
            .value(
                "tags",
                TestValue::Array(vec![TestValue::text("a"), TestValue::text("b")]),
            )
            .value("none", TestValue::Array(Vec::new())),
    );

    let json = json_of(&builder, JsonLayout::Pretty)?;

    assert_eq!(
        json,
        concat!(
            "{\n",
            "  \"$type\": \"Item\",\n",
            "  \"$guid\": \"cbee34fd-6441-4000-b99f-924f7f26e19f\",\n",
            "  \"$path\": \"libs/foundry/records/sample\",\n",
            "  \"$name\": \"Item.Sample\",\n",
            "  \"mass\": 2.5,\n",
            "  \"tags\": [\n",
            "    \"a\",\n",
            "    \"b\"\n",
            "  ],\n",
            "  \"none\": []\n",
            "}\n",
        ),
        "pretty layout"
    );
    Ok(())
}

#[test]
fn floats_have_one_text_form_and_non_finite_ones_are_strings() -> TestResult {
    let builder = one_record(
        TestStruct::new("Item")
            .field("a", TestType::Single)
            .field("b", TestType::Single)
            .field("c", TestType::Single)
            .field("d", TestType::Double)
            .field("e", TestType::Double)
            .field("f", TestType::Double)
            .field("g", TestType::Single)
            .field("h", TestType::Double),
        record("Item")
            .value("a", TestValue::Single(0.1))
            .value("b", TestValue::Single(f32::INFINITY))
            .value("c", TestValue::Single(f32::NEG_INFINITY))
            .value("d", TestValue::Double(f64::NAN))
            .value("e", TestValue::Double(1e21))
            .value("f", TestValue::Double(-0.0))
            .value("g", TestValue::Single(3.0))
            .value("h", TestValue::Double(1.5e-7)),
    );

    let json = json_of(&builder, JsonLayout::Compact)?;

    assert!(
        json.ends_with(
            "\"a\":0.1,\"b\":\"INF\",\"c\":\"-INF\",\"d\":\"NaN\",\"e\":1e21,\"f\":-0,\"g\":3,\"h\":1.5e-7}"
        ),
        "the Record XML text, non-finite values as strings: {json}"
    );
    serde_json::from_str::<serde_json::Value>(&json)?;
    Ok(())
}

#[test]
fn error_values_carry_their_reason() -> TestResult {
    let builder = one_record(
        TestStruct::new("Item")
            .field("count", TestType::Int32)
            .field("label", TestType::Locale),
        record("Item")
            .value("count", TestValue::Int32(3))
            .value("label", TestValue::Raw(vec![0xff; 4])),
    );

    let json = json_of(&builder, JsonLayout::Compact)?;

    assert!(
        json.ends_with(
            "\"count\":3,\"label\":{\"error\":\"offset 4294967295 in the text table is not a valid string\"}}"
        ),
        "an object with the reason at the field: {json}"
    );
    Ok(())
}

#[test]
fn reserved_and_repeated_field_names_are_written_by_their_position() -> TestResult {
    let builder = one_record(
        TestStruct::new("Item")
            .field("count", TestType::Int8)
            .field("$type", TestType::Int8)
            .field("count", TestType::Int8)
            .field("type", TestType::Int8),
        record("Item")
            .value("count", TestValue::Int8(4))
            .value("$type", TestValue::Int8(5))
            .value("type", TestValue::Int8(6)),
    );

    let json = json_of(&builder, JsonLayout::Compact)?;

    assert!(
        json.ends_with(concat!(
            "\"count\":4,",
            "\"$1\":{\"name\":\"$type\",\"value\":5},",
            "\"$2\":{\"name\":\"count\",\"value\":0},",
            "\"type\":6}"
        )),
        "a name starting with $ or taken by an earlier field is kept with its position: {json}"
    );
    serde_json::from_str::<serde_json::Value>(&json)?;
    Ok(())
}

#[test]
fn the_same_record_gives_byte_identical_json() -> TestResult {
    let first = json_of(&scalars(), JsonLayout::Pretty)?;
    let second = json_of(&scalars(), JsonLayout::Pretty)?;

    assert_eq!(first, second, "two runs give the same bytes");
    assert!(!first.contains('\r'), "LF line endings only");
    Ok(())
}
