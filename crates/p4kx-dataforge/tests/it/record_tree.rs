// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Building the canonical Record tree of Records with scalar fields.

use std::error::Error;

use p4kx_dataforge::{DataForge, DataForgeError, Field, RecordTree, Value, ValueError};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLE: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;
const OTHER: u128 = 0x50e8_a9f6_f956_4000_8ace_7c0c_a9a9_1f5d;

fn every_scalar() -> TestStruct {
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
        .field("size", TestType::Enum("Size".to_owned()))
}

fn sample_record() -> TestRecord {
    TestRecord::new(
        SAMPLE,
        "Scalars",
        "Scalars.Sample",
        "libs/foundry/records/sample.xml",
    )
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
    .value("text", TestValue::text("Hello <World> & \"friends\""))
    .value("label", TestValue::text("@item_Name_Sample"))
    .value("id", TestValue::Guid(OTHER))
    .value("size", TestValue::text("Large"))
}

fn database() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .enumeration("Size", ["Small", "Large"])
        .structure(every_scalar())
        .record(sample_record())
}

const SAMPLE_TEXT: &str = "cbee34fd-6441-4000-b99f-924f7f26e19f";

fn tree_of(bytes: &[u8]) -> Result<(DataForge<'_>, &'static str), Box<dyn Error>> {
    Ok((DataForge::open(bytes)?, SAMPLE_TEXT))
}

fn build<'f>(forge: &'f DataForge<'f>, guid: &str) -> Result<RecordTree<'f>, Box<dyn Error>> {
    let record = forge
        .record_by_guid(guid.parse()?)
        .ok_or("no such Record")?;
    Ok(record.tree()?)
}

#[test]
fn a_record_node_carries_type_guid_path_and_name() -> TestResult {
    let bytes = database().build();
    let (forge, guid) = tree_of(&bytes)?;

    let tree = build(&forge, guid)?;

    assert_eq!(tree.type_name(), "Scalars", "type name");
    assert_eq!(tree.guid().to_string(), guid, "GUID");
    assert_eq!(
        tree.record_path(),
        "libs/foundry/records/sample",
        "record path"
    );
    assert_eq!(tree.name(), "Scalars.Sample", "Record name");
    Ok(())
}

#[test]
fn every_scalar_type_is_read_in_declaration_order() -> TestResult {
    let bytes = database().build();
    let (forge, guid) = tree_of(&bytes)?;

    let tree = build(&forge, guid)?;
    let fields: Vec<_> = tree
        .fields()
        .iter()
        .map(|field| (field.name(), field.value().clone()))
        .collect();

    assert_eq!(
        fields,
        [
            ("flag", Value::Bool(true)),
            ("i8", Value::Int8(-8)),
            ("i16", Value::Int16(-1600)),
            ("i32", Value::Int32(-320_000)),
            ("i64", Value::Int64(i64::MIN)),
            ("u8", Value::UInt8(200)),
            ("u16", Value::UInt16(65_000)),
            ("u32", Value::UInt32(4_000_000_000)),
            ("u64", Value::UInt64(u64::MAX)),
            ("single", Value::Single(1.5)),
            ("double", Value::Double(-0.25)),
            ("text", Value::String("Hello <World> & \"friends\"")),
            ("label", Value::Locale("@item_Name_Sample")),
            (
                "id",
                Value::Guid("50e8a9f6-f956-4000-8ace-7c0ca9a91f5d".parse()?)
            ),
            ("size", Value::Enum("Large")),
        ],
        "every scalar with its value, in declaration order"
    );
    assert_eq!(tree.errors().count(), 0, "no error values");
    Ok(())
}

#[test]
fn unset_fields_read_as_zero_values() -> TestResult {
    let bytes = DataForgeBuilder::new()
        .structure(
            TestStruct::new("Plain")
                .field("flag", TestType::Bool)
                .field("count", TestType::UInt32)
                .field("id", TestType::Guid),
        )
        .record(TestRecord::new(SAMPLE, "Plain", "Plain.Zero", "a.xml"))
        .build();
    let (forge, guid) = tree_of(&bytes)?;

    let tree = build(&forge, guid)?;
    let values: Vec<_> = tree.fields().iter().map(Field::value).collect();

    assert_eq!(
        values,
        [
            &Value::Bool(false),
            &Value::UInt32(0),
            &Value::Guid("00000000-0000-0000-0000-000000000000".parse()?),
        ],
        "zero bytes are false, 0 and the empty GUID"
    );
    Ok(())
}

#[test]
fn inherited_fields_come_first() -> TestResult {
    let bytes = DataForgeBuilder::new()
        .structure(TestStruct::new("Base").field("mass", TestType::Single))
        .structure(
            TestStruct::new("Middle")
                .parent("Base")
                .field("crew", TestType::Int16),
        )
        .structure(
            TestStruct::new("Leaf")
                .parent("Middle")
                .field("cargo", TestType::UInt8),
        )
        .record(
            TestRecord::new(SAMPLE, "Leaf", "Leaf.One", "leaf.xml")
                .value("mass", TestValue::Single(2.0))
                .value("crew", TestValue::Int16(3))
                .value("cargo", TestValue::UInt8(4)),
        )
        .build();
    let (forge, guid) = tree_of(&bytes)?;

    let tree = build(&forge, guid)?;
    let fields: Vec<_> = tree
        .fields()
        .iter()
        .map(|field| (field.name(), field.value().clone()))
        .collect();

    assert_eq!(
        fields,
        [
            ("mass", Value::Single(2.0)),
            ("crew", Value::Int16(3)),
            ("cargo", Value::UInt8(4)),
        ],
        "the fields of the root ancestor first"
    );
    Ok(())
}

#[test]
fn several_records_of_one_struct_read_their_own_instances_in_v6_and_v8() -> TestResult {
    for version in [6, 8] {
        let bytes = DataForgeBuilder::new()
            .version(version)
            .structure(TestStruct::new("Counter").field("count", TestType::Int32))
            .record(
                TestRecord::new(SAMPLE, "Counter", "Counter.A", "a.xml")
                    .tag("Design")
                    .value("count", TestValue::Int32(1)),
            )
            .record(
                TestRecord::new(OTHER, "Counter", "Counter.B", "b.xml")
                    .value("count", TestValue::Int32(2)),
            )
            .build();
        let forge = DataForge::open(&bytes)?;

        let counts = forge
            .records()
            .map(|record| Ok(record.tree()?.fields().first().map(|f| f.value().clone())))
            .collect::<Result<Vec<_>, DataForgeError>>()?;

        assert_eq!(
            counts,
            [Some(Value::Int32(1)), Some(Value::Int32(2))],
            "each Record of version {version} reads its own instance"
        );
    }
    Ok(())
}

#[test]
fn out_of_range_strings_and_enums_become_error_values() -> TestResult {
    let bytes = DataForgeBuilder::new()
        .enumeration("Size", ["Small", "Large"])
        .structure(every_scalar().field("missing", TestType::Enum("Missing".to_owned())))
        .record(
            sample_record()
                .value("text", TestValue::Raw(u32::MAX.to_le_bytes().to_vec()))
                .value(
                    "label",
                    TestValue::Raw(0x00ff_ffff_u32.to_le_bytes().to_vec()),
                )
                .value(
                    "size",
                    TestValue::Raw(0x7fff_ffff_u32.to_le_bytes().to_vec()),
                )
                .value("flag", TestValue::Raw(vec![2])),
        )
        .build();
    let (forge, guid) = tree_of(&bytes)?;

    let tree = build(&forge, guid)?;
    let errors: Vec<_> = tree.errors().map(|(name, error)| (name, *error)).collect();

    assert_eq!(
        errors,
        [
            ("flag", ValueError::InvalidBool { byte: 2 }),
            (
                "text",
                ValueError::InvalidString {
                    table: "text",
                    offset: u32::MAX
                }
            ),
            (
                "label",
                ValueError::InvalidString {
                    table: "text",
                    offset: 0x00ff_ffff
                }
            ),
            (
                "size",
                ValueError::InvalidString {
                    table: "text",
                    offset: 0x7fff_ffff
                }
            ),
            ("missing", ValueError::UnknownEnum { index: u16::MAX }),
        ],
        "each broken value is an error at its field"
    );
    assert_eq!(
        tree.fields().get(1).map(Field::value),
        Some(&Value::Int8(-8)),
        "the other fields still build"
    );
    assert_eq!(
        tree.fields().len(),
        16,
        "every field is present, the broken ones included"
    );
    Ok(())
}
