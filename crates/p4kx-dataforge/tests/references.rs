//! References and weak pointers in the Record tree: links, never embedded content.

use std::error::Error;

use p4kx_dataforge::{DataForge, Field, RecordTree, Reference, Value, ValueError};
use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLE: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;
const OTHER: u128 = 0x50e8_a9f6_f956_4000_8ace_7c0c_a9a9_1f5d;
const MISSING: u128 = 0x0000_0001_0002_0003_0004_0000_0000_0005;

fn value<'t, 'f>(tree: &'t RecordTree<'f>, name: &str) -> Result<&'t Value<'f>, String> {
    tree.fields()
        .iter()
        .find(|field| field.name() == name)
        .map(Field::value)
        .ok_or_else(|| format!("no field {name}"))
}

fn link<'t, 'f>(value: &'t Value<'f>) -> Result<&'t Reference<'f>, String> {
    match value {
        Value::Reference(link) => Ok(link),
        other => Err(format!("no Reference: {other:?}")),
    }
}

fn first_tree<'f>(forge: &'f DataForge<'f>) -> Result<RecordTree<'f>, Box<dyn Error>> {
    Ok(forge.records().next().ok_or("no Record")?.tree()?)
}

#[test]
fn a_reference_carries_guid_record_path_and_type_of_its_target() -> TestResult {
    let bytes = DataForgeBuilder::new()
        .structure(TestStruct::new("Item").field("mass", TestType::Single))
        .structure(
            TestStruct::new("Ship")
                .field("cargo", TestType::Reference)
                .field("empty", TestType::Reference)
                .field("lost", TestType::Reference),
        )
        .record(
            TestRecord::new(SAMPLE, "Ship", "Ship.One", "ships/one.xml")
                .value("cargo", TestValue::Reference(OTHER))
                .value("lost", TestValue::Reference(MISSING)),
        )
        .record(
            TestRecord::new(OTHER, "Item", "Item.Box", "items/box.xml")
                .value("mass", TestValue::Single(4.0)),
        )
        .build();
    let forge = DataForge::open(&bytes)?;

    let tree = first_tree(&forge)?;

    let cargo = link(value(&tree, "cargo")?)?;
    assert_eq!(
        cargo.guid().to_string(),
        "50e8a9f6-f956-4000-8ace-7c0ca9a91f5d",
        "GUID"
    );
    assert_eq!(cargo.record_path(), "items/box", "record path without .xml");
    assert_eq!(cargo.type_name(), "Item", "type");
    assert_eq!(cargo.location(), None, "the Record itself");
    assert_eq!(value(&tree, "empty")?, &Value::Null, "a null Reference");
    assert_eq!(
        value(&tree, "lost")?,
        &Value::Error(ValueError::DanglingReference {
            guid: "00000001-0002-0003-0004-000000000005".parse()?
        }),
        "a dangling Reference is an error at its field"
    );
    Ok(())
}

fn scene() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(
            TestStruct::new("Node")
                .field("name", TestType::String)
                .field("parent", TestType::WeakPointer("Node".to_owned())),
        )
        .structure(
            TestStruct::new("Meta").field("owner", TestType::StrongPointer("Node".to_owned())),
        )
        .structure(
            TestStruct::new("Scene")
                .field("root", TestType::StrongPointer("Node".to_owned()))
                .array("nodes", TestType::StrongPointer("Node".to_owned()))
                .field("meta", TestType::Class("Meta".to_owned()))
                .array("links", TestType::WeakPointer("Node".to_owned()))
                .field("other", TestType::WeakPointer("Scene".to_owned()))
                .field("stray", TestType::WeakPointer("Node".to_owned()))
                .field("myself", TestType::WeakPointer("Scene".to_owned())),
        )
        .instance(
            "root",
            TestInstance::new("Node").value("name", TestValue::text("root")),
        )
        .instance(
            "child",
            TestInstance::new("Node")
                .value("name", TestValue::text("child"))
                .value("parent", TestValue::pointer("root")),
        )
        .instance("owner", TestInstance::new("Node"))
        .instance("loose", TestInstance::new("Node"))
}

fn scene_records() -> Vec<u8> {
    scene()
        .record(
            TestRecord::new(SAMPLE, "Scene", "Scene.One", "scenes/one.xml")
                .value("root", TestValue::pointer("root"))
                .value(
                    "nodes",
                    TestValue::Array(vec![TestValue::Null, TestValue::pointer("child")]),
                )
                .value(
                    "meta",
                    TestValue::Struct(
                        TestInstance::new("Meta").value("owner", TestValue::pointer("owner")),
                    ),
                )
                .value(
                    "links",
                    TestValue::Array(vec![
                        TestValue::pointer("child"),
                        TestValue::pointer("owner"),
                        TestValue::Null,
                    ]),
                )
                .value("other", TestValue::pointer("Scene.Two"))
                .value("stray", TestValue::pointer("loose"))
                .value("myself", TestValue::pointer("Scene.One")),
        )
        .record(TestRecord::new(
            OTHER,
            "Scene",
            "Scene.Two",
            "scenes/two.xml",
        ))
        .build()
}

#[test]
fn weak_pointers_become_references_into_the_record() -> TestResult {
    let bytes = scene_records();
    let forge = DataForge::open(&bytes)?;

    let tree = first_tree(&forge)?;

    let Value::Array(links) = value(&tree, "links")? else {
        return Err("links is an array".into());
    };
    let described: Vec<_> = links
        .iter()
        .map(|item| match item {
            Value::Reference(link) => format!(
                "{} {} {} {:?}",
                link.guid(),
                link.record_path(),
                link.type_name(),
                link.location()
            ),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        described,
        [
            "cbee34fd-6441-4000-b99f-924f7f26e19f scenes/one Node Some(\"nodes[1]\")",
            "cbee34fd-6441-4000-b99f-924f7f26e19f scenes/one Node Some(\"meta.owner\")",
            "Null",
        ],
        "links into the Record with the location of the target"
    );
    let Value::Array(nodes) = value(&tree, "nodes")? else {
        return Err("nodes is an array".into());
    };
    let Some(Value::Struct(child)) = nodes.get(1) else {
        return Err("the child is owned".into());
    };
    let parent = child
        .fields()
        .iter()
        .find(|field| field.name() == "parent")
        .ok_or("parent")?;
    assert_eq!(
        link(parent.value())?.location(),
        Some("root"),
        "a nested weak pointer"
    );
    Ok(())
}

#[test]
fn weak_pointers_to_records_and_outside_the_record() -> TestResult {
    let bytes = scene_records();
    let forge = DataForge::open(&bytes)?;

    let tree = first_tree(&forge)?;

    let other = link(value(&tree, "other")?)?;
    assert_eq!(
        (other.record_path(), other.type_name(), other.location()),
        ("scenes/two", "Scene", None),
        "a weak pointer to another Record is a Reference to it"
    );
    let myself = link(value(&tree, "myself")?)?;
    assert_eq!(
        (myself.record_path(), myself.location()),
        ("scenes/one", None),
        "a weak pointer to the Record itself"
    );
    assert!(
        matches!(
            value(&tree, "stray")?,
            Value::Error(ValueError::OutsideRecord { .. })
        ),
        "a weak pointer outside the Record is an error: {:?}",
        value(&tree, "stray")?
    );
    Ok(())
}
