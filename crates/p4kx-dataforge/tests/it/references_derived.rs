// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Weak pointers whose holder is a derived-type instance, and weak pointers into class arrays or
//! into a non-root part of another Record.

use crate::versions::{TestResult, in_both_versions};
use p4kx_dataforge::{DataForge, Location, Reference, Step, Value, ValueError};
use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestStruct, TestType, TestValue};

const SCENE: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;
const OTHER: u128 = 0x50e8_a9f6_f956_4000_8ace_7c0c_a9a9_1f5d;

fn node(name: &str) -> TestType {
    TestType::StrongPointer(name.to_owned())
}

fn weak_node() -> TestType {
    TestType::WeakPointer("Node".to_owned())
}

fn builder() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(TestStruct::new("Node").field("name", TestType::String))
        .structure(
            TestStruct::new("Special")
                .parent("Node")
                .field("start", weak_node())
                .field("later", weak_node())
                .field("inner", weak_node()),
        )
        .structure(
            TestStruct::new("Scene")
                .array("fragments", TestType::Class("Node".to_owned()))
                .array("steps", node("Node"))
                .field("holder", node("Node"))
                .field("peer", weak_node()),
        )
        .instance("holder", holder())
        .instance("step", TestInstance::new("Special"))
        .record(scene())
        .record(other())
}

fn holder() -> TestInstance {
    TestInstance::new("Special")
        .value("start", TestValue::pointer("first"))
        .value("later", TestValue::pointer("second"))
        .value("inner", TestValue::pointer("step"))
}

fn scene() -> TestRecord {
    TestRecord::new(SCENE, "Scene", "Scene.One", "scenes/one.xml")
        .value(
            "fragments",
            TestValue::Array(vec![
                TestValue::Struct(TestInstance::new("Node").named("first")),
                TestValue::Struct(TestInstance::new("Node").named("second")),
            ]),
        )
        .value("steps", TestValue::Array(vec![TestValue::pointer("step")]))
        .value("holder", TestValue::pointer("holder"))
        .value("peer", TestValue::pointer("step"))
}

fn other() -> TestRecord {
    TestRecord::new(OTHER, "Scene", "Scene.Two", "scenes/two.xml")
        .value("peer", TestValue::pointer("step"))
}

fn link<'t, 'f>(value: Option<&'t Value<'f>>) -> Result<&'t Reference<'f>, String> {
    match value {
        Some(Value::Reference(link)) => Ok(link),
        other => Err(format!("no Reference: {other:?}")),
    }
}

#[test]
fn a_derived_holder_behind_a_base_pointer_links_into_class_arrays_and_strong_targets() -> TestResult
{
    in_both_versions(&builder(), |tree| {
        let Some(Value::Struct(holder)) = tree.field("holder") else {
            return Err("the holder is owned".into());
        };
        assert_eq!(
            holder.type_name(),
            "Special",
            "the actual struct, not the base"
        );
        let start = link(holder.field("start"))?;
        let later = link(holder.field("later"))?;
        let inner = link(holder.field("inner"))?;
        assert_eq!(
            start.location().map(Location::steps),
            Some(&[Step::Field("fragments"), Step::Element(0)][..]),
            "the first class array element"
        );
        assert_eq!(
            later.location().map(ToString::to_string),
            Some("fragments[1]".to_owned()),
            "the second class array element"
        );
        assert_eq!(
            inner.location().map(ToString::to_string),
            Some("steps[0]".to_owned()),
            "an instance a strong pointer owns"
        );
        assert_eq!(
            (start.record_path(), start.type_name()),
            ("scenes/one", "Node"),
            "same Record"
        );
        Ok(())
    })
}

#[test]
fn a_weak_pointer_into_a_non_root_part_of_another_record_is_outside_the_record() -> TestResult {
    in_both_versions(&builder(), |tree| {
        assert_eq!(
            link(tree.field("peer"))?
                .location()
                .map(ToString::to_string),
            Some("steps[0]".to_owned()),
            "the Record that owns the target links to it"
        );
        Ok(())
    })?;
    for version in [6, 8] {
        let bytes = builder().version(version).build();
        let forge = DataForge::open(&bytes)?;
        let second = forge.records().nth(1).ok_or("no second Record")?.tree()?;
        assert!(
            matches!(
                second.field("peer"),
                Some(Value::Error(ValueError::OutsideRecord { .. }))
            ),
            "version {version}: {:?}",
            second.field("peer")
        );
    }
    Ok(())
}
