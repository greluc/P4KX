// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Writing resolved localisation texts into Record XML and Record JSON.

use std::borrow::Cow;
use std::error::Error;

use p4kx_dataforge::{DataForge, JsonLayout, RecordJson, RecordXml, Resolution, TextResolver};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const GUID: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;

struct Texts;

impl TextResolver for Texts {
    fn resolve<'a>(&'a self, key: &str) -> Resolution<'a> {
        match key {
            "@item_Name" => Resolution::Text(Cow::Borrowed("Crate & \"Co\"")),
            "@item_Desc" => Resolution::Text(Cow::Borrowed("Line one\nLine two")),
            "@LOC_EMPTY" => Resolution::Keep,
            _ => Resolution::Missing,
        }
    }
}

fn builder() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(
            TestStruct::new("Item")
                .field("name", TestType::Locale)
                .field("gone", TestType::Locale)
                .field("blank", TestType::Locale)
                .field("unset", TestType::Locale)
                .field("path", TestType::Locale)
                .field("text", TestType::String)
                .array("lines", TestType::Locale),
        )
        .record(
            TestRecord::new(
                GUID,
                "Item",
                "Item.Sample",
                "libs/foundry/records/sample.xml",
            )
            .value("name", TestValue::text("@item_Name"))
            .value("gone", TestValue::text("@item_Gone"))
            .value("blank", TestValue::text("@LOC_EMPTY"))
            .value("unset", TestValue::text(""))
            .value("path", TestValue::text("@item_Desc"))
            .value("text", TestValue::text("@item_Name"))
            .value(
                "lines",
                TestValue::Array(vec![
                    TestValue::text("@item_Name"),
                    TestValue::text("@item_Gone"),
                ]),
            ),
        )
}

fn xml(texts: Option<&dyn TextResolver>) -> Result<String, Box<dyn Error>> {
    let bytes = builder().build();
    let forge = DataForge::open(&bytes)?;
    let tree = forge.records().next().ok_or("no Record")?.tree()?;
    let mut out = Vec::new();
    RecordXml::new().write_annotated(&tree, texts.into(), &mut out)?;
    Ok(String::from_utf8(out)?)
}

fn json(texts: Option<&dyn TextResolver>) -> Result<String, Box<dyn Error>> {
    let bytes = builder().build();
    let forge = DataForge::open(&bytes)?;
    let tree = forge.records().next().ok_or("no Record")?.tree()?;
    let mut out = Vec::new();
    RecordJson::new(JsonLayout::Compact).write_annotated(&tree, texts.into(), &mut out)?;
    Ok(String::from_utf8(out)?)
}

#[test]
fn without_texts_the_output_is_that_of_the_plain_writers() -> TestResult {
    let bytes = builder().build();
    let forge = DataForge::open(&bytes)?;
    let tree = forge.records().next().ok_or("no Record")?.tree()?;
    let (mut plain_xml, mut plain_json) = (Vec::new(), Vec::new());
    tree.write_xml(&mut plain_xml)?;
    RecordJson::new(JsonLayout::Compact).write(&tree, &mut plain_json)?;

    assert_eq!(xml(None)?.as_bytes(), plain_xml);
    assert_eq!(json(None)?.as_bytes(), plain_json);
    assert!(
        !xml(None)?.contains("p4kx:text") && !xml(None)?.contains("missing"),
        "no annotation without a language"
    );
    assert!(
        !json(None)?.contains("\"key\""),
        "no annotation without a language"
    );
    Ok(())
}

#[test]
fn xml_annotates_attributes_elements_and_array_values() -> TestResult {
    let xml = xml(Some(&Texts))?;

    assert_eq!(
        xml,
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<Item xmlns:p4kx=\"https://github.com/p4kx-org/p4kx/record-xml\" ",
            "guid=\"cbee34fd-6441-4000-b99f-924f7f26e19f\" path=\"libs/foundry/records/sample\" ",
            "p4kx:name=\"Item.Sample\" name=\"@item_Name\" p4kx:text-name=\"Crate &amp; &quot;Co&quot;\" ",
            "gone=\"@item_Gone\" p4kx:missing-gone=\"true\" ",
            "blank=\"@LOC_EMPTY\" unset=\"\" text=\"@item_Name\">\n",
            "  <p4kx:field name=\"path\" value=\"@item_Desc\" text=\"Line one&#10;Line two\"/>\n",
            "  <lines>\n",
            "    <p4kx:value value=\"@item_Name\" text=\"Crate &amp; &quot;Co&quot;\"/>\n",
            "    <p4kx:value value=\"@item_Gone\" missing=\"true\"/>\n",
            "  </lines>\n",
            "</Item>\n",
        ),
        "text and missing marks next to the key; placeholders, empty keys and strings stay alone"
    );
    roxmltree::Document::parse(&xml)?;
    Ok(())
}

#[test]
fn json_annotates_every_localisation_key_with_an_object() -> TestResult {
    let json = json(Some(&Texts))?;

    assert_eq!(
        json,
        concat!(
            "{\"$type\":\"Item\",\"$guid\":\"cbee34fd-6441-4000-b99f-924f7f26e19f\",",
            "\"$path\":\"libs/foundry/records/sample\",\"$name\":\"Item.Sample\",",
            "\"name\":{\"key\":\"@item_Name\",\"text\":\"Crate & \\\"Co\\\"\"},",
            "\"gone\":{\"key\":\"@item_Gone\",\"missing\":true},",
            "\"blank\":\"@LOC_EMPTY\",\"unset\":\"\",\"text\":\"@item_Name\",",
            "\"path\":{\"key\":\"@item_Desc\",\"text\":\"Line one\\nLine two\"},",
            "\"lines\":[{\"key\":\"@item_Name\",\"text\":\"Crate & \\\"Co\\\"\"},",
            "{\"key\":\"@item_Gone\",\"missing\":true}]}"
        ),
    );
    serde_json::from_str::<serde_json::Value>(&json)?;
    Ok(())
}

#[test]
fn the_same_texts_give_byte_identical_output() -> TestResult {
    assert_eq!(xml(Some(&Texts))?, xml(Some(&Texts))?);
    assert_eq!(json(Some(&Texts))?, json(Some(&Texts))?);
    Ok(())
}
