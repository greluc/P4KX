// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Record tag in the flat value export.

use std::error::Error;

use p4kx_dataforge::{Annotations, DataForge, FlatFormat, FlatValues, TreeBuilder};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const TAGGED: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;
const PLAIN: u128 = 0x50e8_a9f6_f956_4000_8ace_7c0c_a9a9_1f5d;
const TAGGED_HEAD: &str = "cbee34fd-6441-4000-b99f-924f7f26e19f,Item.Box,libs/items/box,";
const PLAIN_HEAD: &str = "50e8a9f6-f956-4000-8ace-7c0ca9a91f5d,Item.Crate,libs/items/crate,";

fn bytes(version: u32, tag: &str) -> Vec<u8> {
    DataForgeBuilder::new()
        .version(version)
        .structure(TestStruct::new("Item").field("mass", TestType::Int32))
        .record(
            TestRecord::new(TAGGED, "Item", "Item.Box", "libs/items/box.xml")
                .tag(tag)
                .value("mass", TestValue::Int32(3)),
        )
        .record(
            TestRecord::new(PLAIN, "Item", "Item.Crate", "libs/items/crate.xml")
                .value("mass", TestValue::Int32(4)),
        )
        .build()
}

fn flat(version: u32, tag: &str, format: FlatFormat) -> Result<String, Box<dyn Error>> {
    let bytes = bytes(version, tag);
    let forge = DataForge::open(&bytes)?;
    let mut writer = FlatValues::new(format);
    let mut out = Vec::new();
    for record in forge.records() {
        let tree = TreeBuilder::new().build(&record)?;
        writer.write_annotated(&tree, Annotations::none(), &mut out)?;
    }
    Ok(String::from_utf8(out)?)
}

fn objects(text: &str) -> Result<Vec<serde_json::Value>, Box<dyn Error>> {
    Ok(text
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?)
}

#[test]
fn a_version_8_record_with_a_tag_has_it_in_the_ninth_csv_column() -> TestResult {
    let text = flat(8, "SystemsDesign", FlatFormat::CsvRaw)?;

    assert!(
        text.contains(&format!("{TAGGED_HEAD}mass,,int,3,,SystemsDesign\r\n")),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_version_8_record_without_a_tag_has_an_empty_cell() -> TestResult {
    let text = flat(8, "SystemsDesign", FlatFormat::CsvRaw)?;

    assert!(
        text.contains(&format!("{PLAIN_HEAD}mass,,int,4,,\r\n")),
        "{text}"
    );
    Ok(())
}

#[test]
fn an_empty_tag_counts_as_no_tag() -> TestResult {
    let text = flat(8, "", FlatFormat::CsvRaw)?;

    assert!(
        text.contains(&format!("{TAGGED_HEAD}mass,,int,3,,\r\n")),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_version_6_record_has_an_empty_cell() -> TestResult {
    let text = flat(6, "SystemsDesign", FlatFormat::CsvRaw)?;

    assert!(
        text.contains(&format!("{TAGGED_HEAD}mass,,int,3,,\r\n")),
        "{text}"
    );
    assert!(!text.contains("SystemsDesign"), "{text}");
    Ok(())
}

#[test]
fn the_csv_tag_is_quoted_and_neutralised_like_any_text() -> TestResult {
    let text = flat(8, "=a,b", FlatFormat::Csv)?;
    assert!(text.contains(",int,3,,\"'=a,b\"\r\n"), "{text}");

    let raw = flat(8, "=a,b", FlatFormat::CsvRaw)?;
    assert!(raw.contains(",int,3,,\"=a,b\"\r\n"), "{raw}");
    Ok(())
}

#[test]
fn ndjson_has_the_record_tag_key_only_for_a_tagged_record() -> TestResult {
    let rows = objects(&flat(8, "SystemsDesign", FlatFormat::Ndjson)?)?;

    let tagged = rows
        .iter()
        .find(|row| row["record_name"] == "Item.Box")
        .ok_or("tagged")?;
    assert_eq!(tagged["record_tag"], "SystemsDesign");
    let plain = rows
        .iter()
        .find(|row| row["record_name"] == "Item.Crate")
        .ok_or("plain")?;
    assert!(plain.get("record_tag").is_none());
    Ok(())
}

#[test]
fn ndjson_of_a_version_6_database_has_no_record_tag() -> TestResult {
    let rows = objects(&flat(6, "SystemsDesign", FlatFormat::Ndjson)?)?;

    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.get("record_tag").is_none()));
    Ok(())
}
