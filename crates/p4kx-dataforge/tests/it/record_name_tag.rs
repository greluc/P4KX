// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Record name and the Record tag in Record XML and Record JSON.

use std::error::Error;

use p4kx_dataforge::{DataForge, JsonLayout, RecordJson, RecordXml};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const GUID: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;
const OTHER: u128 = 0x50e8_a9f6_f956_4000_8ace_7c0c_a9a9_1f5d;

fn sample(version: u32) -> DataForgeBuilder {
    DataForgeBuilder::new()
        .version(version)
        .structure(
            TestStruct::new("Item")
                .field("name", TestType::String)
                .field("$name", TestType::String)
                .field("mass", TestType::Int32)
                .field("tag", TestType::String)
                .field("$tag", TestType::String),
        )
        .record(
            TestRecord::new(GUID, "Item", "Item.Box", "libs/items/box.xml")
                .tag("SystemsDesign")
                .value("name", TestValue::text("Box"))
                .value("$name", TestValue::text("Dollar"))
                .value("mass", TestValue::Int32(3))
                .value("tag", TestValue::text("Field"))
                .value("$tag", TestValue::text("Dollar tag")),
        )
        .record(TestRecord::new(
            OTHER,
            "Item",
            "Item.Crate",
            "libs/items/box.xml",
        ))
}

fn texts(version: u32, index: usize) -> Result<(String, String), Box<dyn Error>> {
    let bytes = sample(version).build();
    let forge = DataForge::open(&bytes)?;
    let tree = forge.records().nth(index).ok_or("no Record")?.tree()?;
    let (mut xml, mut json) = (Vec::new(), Vec::new());
    RecordXml::new().write(&tree, &mut xml)?;
    RecordJson::new(JsonLayout::Compact).write(&tree, &mut json)?;
    Ok((String::from_utf8(xml)?, String::from_utf8(json)?))
}

#[test]
fn a_version_8_record_has_its_name_and_tag_in_xml_and_json() -> TestResult {
    let (xml, json) = texts(8, 0)?;

    assert!(
        xml.contains(
            "<Item xmlns:p4kx=\"https://github.com/p4kx-org/p4kx/record-xml\" \
             guid=\"cbee34fd-6441-4000-b99f-924f7f26e19f\" path=\"libs/items/box\" \
             p4kx:name=\"Item.Box\" p4kx:tag=\"SystemsDesign\" name=\"Box\" mass=\"3\" tag=\"Field\">"
        ),
        "XML root: {xml}"
    );
    assert!(
        json.starts_with(concat!(
            "{\"$type\":\"Item\",\"$guid\":\"cbee34fd-6441-4000-b99f-924f7f26e19f\",",
            "\"$path\":\"libs/items/box\",\"$name\":\"Item.Box\",\"$tag\":\"SystemsDesign\","
        )),
        "JSON header: {json}"
    );
    Ok(())
}

#[test]
fn a_version_6_record_has_the_name_and_no_tag() -> TestResult {
    let (xml, json) = texts(6, 0)?;

    assert!(xml.contains("p4kx:name=\"Item.Box\""), "XML name: {xml}");
    assert!(!xml.contains("p4kx:tag"), "XML tag: {xml}");
    assert!(json.contains("\"$name\":\"Item.Box\""), "JSON name: {json}");
    assert!(!json.contains("\"$tag\":\""), "JSON tag: {json}");
    Ok(())
}

#[test]
fn a_version_8_record_without_tag_has_no_tag() -> TestResult {
    let (xml, json) = texts(8, 1)?;

    assert!(xml.contains("p4kx:name=\"Item.Crate\""), "XML name: {xml}");
    assert!(!xml.contains("p4kx:tag"), "XML tag: {xml}");
    assert!(!json.contains("\"$tag\":\""), "JSON tag: {json}");
    Ok(())
}

#[test]
fn fields_called_name_keep_their_own_place() -> TestResult {
    let (xml, json) = texts(8, 0)?;

    assert!(xml.contains(" name=\"Box\""), "XML field: {xml}");
    assert!(json.contains("\"name\":\"Box\""), "JSON field: {json}");
    assert!(
        json.contains("\"$1\":{\"name\":\"$name\",\"value\":\"Dollar\"}"),
        "the field `$name` is escaped: {json}"
    );
    assert!(json.contains("\"tag\":\"Field\""), "JSON field tag: {json}");
    assert!(
        json.contains("\"$4\":{\"name\":\"$tag\",\"value\":\"Dollar tag\"}"),
        "the field `$tag` is escaped: {json}"
    );
    assert!(xml.contains(" tag=\"Field\""), "XML field tag: {xml}");
    assert!(
        xml.contains("<p4kx:field name=\"$tag\" value=\"Dollar tag\"/>"),
        "XML field `$tag`: {xml}"
    );
    Ok(())
}
