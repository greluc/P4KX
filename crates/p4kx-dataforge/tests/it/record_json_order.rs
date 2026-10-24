// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Record JSON keeps the field order of the Record XML of the same Record.

use std::error::Error;
use std::fmt;

use p4kx_dataforge::{DataForge, JsonLayout, RecordJson, RecordXml};
use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestStruct, TestType, TestValue};
use serde::Deserializer as _;
use serde::de::{IgnoredAny, MapAccess, Visitor};

type TestResult = Result<(), Box<dyn Error>>;

/// Collects the keys of a JSON object in the order they are written.
struct Keys;

impl<'de> Visitor<'de> for Keys {
    type Value = Vec<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut keys = Vec::new();
        while let Some((key, IgnoredAny)) = map.next_entry::<String, IgnoredAny>()? {
            keys.push(key);
        }
        Ok(keys)
    }
}

fn record() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .enumeration("Size", ["Small", "Large"])
        .structure(TestStruct::new("Wheel").field("size", TestType::Int32))
        .structure(
            TestStruct::new("Item")
                .array("tags", TestType::String)
                .field("count", TestType::Int32)
                .field("wheel", TestType::Class("Wheel".to_owned()))
                .field("label", TestType::Locale)
                .field("type", TestType::Enum("Size".to_owned()))
                .field("maker", TestType::Reference)
                .field("mass", TestType::Single)
                .field("spare", TestType::StrongPointer("Wheel".to_owned()))
                .field("bad:name", TestType::Bool)
                .field("flag", TestType::Bool),
        )
        .record(
            TestRecord::new(1, "Item", "Item.One", "libs/foundry/records/one.xml")
                .value("tags", TestValue::Array(vec![TestValue::text("a")]))
                .value("count", TestValue::Int32(3))
                .value(
                    "wheel",
                    TestValue::Struct(
                        TestInstance::new("Wheel").value("size", TestValue::Int32(1)),
                    ),
                )
                .value("label", TestValue::Raw(vec![0xff; 4]))
                .value("type", TestValue::text("Large"))
                .value("maker", TestValue::Reference(1))
                .value("mass", TestValue::Single(1.0))
                .value("bad:name", TestValue::Bool(true))
                .value("flag", TestValue::Bool(false)),
        )
}

/// Returns the field names in the order the Record XML writes them: the attributes after `guid`
/// and `path`, then the child elements by field name.
fn xml_field_order(xml: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let document = roxmltree::Document::parse(xml)?;
    let root = document.root_element();
    let mut names: Vec<String> = root
        .attributes()
        .map(|attribute| attribute.name().to_owned())
        .filter(|name| !matches!(name.as_str(), "guid" | "path"))
        .collect();
    for child in root.children().filter(roxmltree::Node::is_element) {
        let name = child
            .attribute("name")
            .or_else(|| child.attribute("field"))
            .unwrap_or_else(|| child.tag_name().name());
        names.push(name.to_owned());
    }
    Ok(names)
}

#[test]
fn json_keys_follow_the_record_xml_field_order() -> TestResult {
    let bytes = record().build();
    let forge = DataForge::open(&bytes)?;
    let tree = forge.records().next().ok_or("no Record")?.tree()?;
    let mut xml = Vec::new();
    RecordXml::new().write(&tree, &mut xml)?;
    let mut json = Vec::new();
    RecordJson::new(JsonLayout::Compact).write(&tree, &mut json)?;

    let expected = xml_field_order(std::str::from_utf8(&xml)?)?;
    let keys = serde_json::Deserializer::from_slice(&json).deserialize_map(Keys)?;

    assert_eq!(
        keys.get(..3),
        Some(&["$type", "$guid", "$path"].map(str::to_owned)[..]),
        "the Record first"
    );
    assert_eq!(keys.get(3..), Some(&expected[..]), "the order of {xml:?}");
    assert_ne!(
        expected.first().map(String::as_str),
        Some("tags"),
        "the XML order is not the declaration order"
    );
    Ok(())
}
