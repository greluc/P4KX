// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Mutation self-test of the value comparison: every kind of change to a reference Record must
//! be detected, and the differences that are no change must not be.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::type_complexity,
    reason = "the helpers mutate synthetic data; a value that is not there fails the test"
)]

use serde_json::{Value, json};

use super::samples::{BIG, ours, reference_json};
use super::starbreaker_json::record;
use super::tally::Compared;
use super::value_tree::{JsonNumber, Node, RecordDoc};
use super::value_tree_compare::compare_records;

fn compared(reference: &Value) -> Compared {
    let theirs = record(&reference.to_string()).expect("a Record");
    compare_records(&ours(), &theirs)
}

fn at<'v>(reference: &'v mut Value, pointer: &str) -> &'v mut Value {
    reference
        .pointer_mut(&format!("/_RecordValue_{pointer}"))
        .expect("the synthetic Record has this value")
}

fn set(pointer: &'static str, replacement: Value) -> Box<dyn Fn(&mut Value)> {
    Box::new(move |reference| *at(reference, pointer) = replacement.clone())
}

fn next_single(pointer: &'static str) -> Box<dyn Fn(&mut Value)> {
    Box::new(move |reference| {
        let value = at(reference, pointer);
        let single = value.as_f64().expect("a number") as f32;
        *value = json!(f64::from(single.next_up()));
    })
}

fn next_double(pointer: &'static str) -> Box<dyn Fn(&mut Value)> {
    Box::new(move |reference| {
        let value = at(reference, pointer);
        *value = json!(value.as_f64().expect("a number").next_up());
    })
}

fn remove(pointer: &'static str) -> Box<dyn Fn(&mut Value)> {
    let (parent, last) = pointer.rsplit_once('/').expect("a pointer with a parent");
    Box::new(move |reference| match at(reference, parent) {
        Value::Array(items) => {
            items.remove(last.parse::<usize>().expect("an index"));
        }
        Value::Object(map) => {
            map.remove(last).expect("the field exists");
        }
        other => panic!("{other} has no children"),
    })
}

fn insert(pointer: &'static str, key: &'static str, added: Value) -> Box<dyn Fn(&mut Value)> {
    Box::new(move |reference| match at(reference, pointer) {
        Value::Array(items) => items.push(added.clone()),
        Value::Object(map) => {
            map.insert(key.to_owned(), added.clone());
        }
        other => panic!("{other} has no children"),
    })
}

fn mutations() -> Vec<(&'static str, Box<dyn Fn(&mut Value)>)> {
    vec![
        ("integer plus one", set("/count", json!(-41))),
        ("large unsigned plus one", set("/big", json!(BIG + 1))),
        ("single precision, smallest step", next_single("/ratio")),
        ("double precision, smallest step", next_double("/precise")),
        ("zero becomes a small number", set("/zero", json!(1e-30))),
        ("boolean flipped", set("/flag", json!(false))),
        ("text changed", set("/label", json!("Hallo"))),
        (
            "enum option swapped for another",
            set("/size", json!("Small")),
        ),
        (
            "texts swapped",
            Box::new(|reference| {
                let label = at(reference, "/label").take();
                let size = std::mem::replace(at(reference, "/size"), label);
                *at(reference, "/label") = size;
            }),
        ),
        (
            "GUID value changed",
            set("/id", json!("50e8a9f6-f956-4000-8ace-7c0ca9a91f5e")),
        ),
        (
            "Reference retargeted",
            set("/link/_RecordId_", json!(super::samples::SAMPLE_GUID)),
        ),
        ("value of another kind", set("/count", json!("-42"))),
        ("null instead of a value", set("/label", Value::Null)),
        ("array element deleted", remove("/tags/1")),
        ("array element invented", insert("/tags", "", json!("c"))),
        ("array element changed", set("/points/1/x", json!(3.0))),
        ("nested value changed", set("/inner/x", json!(2.75))),
        ("struct type changed", set("/inner/_Type_", json!("Other"))),
        ("field deleted", remove("/count")),
        ("field invented", insert("", "extra", json!(1))),
        ("nested field invented", insert("/inner", "extra", json!(1))),
        (
            "Record name changed",
            Box::new(|reference| {
                reference["_RecordName_"] = json!("Sample.Two");
            }),
        ),
        (
            "Record tag changed",
            Box::new(|reference| {
                reference["_RecordTag_"] = json!("Other");
            }),
        ),
        (
            "Record GUID changed",
            Box::new(|reference| {
                reference["_RecordId_"] = json!("50e8a9f6-f956-4000-8ace-7c0ca9a91f5d");
            }),
        ),
    ]
}

#[test]
fn the_unchanged_reference_is_equal() {
    assert_eq!(compared(&reference_json(&ours())), Compared::Equal);
}

#[test]
fn every_kind_of_change_to_a_reference_record_is_detected() {
    for (name, mutate) in mutations() {
        let mut reference = reference_json(&ours());
        mutate(&mut reference);
        assert!(
            matches!(compared(&reference), Compared::Different(_)),
            "{name} is not detected: {:?}",
            compared(&reference)
        );
    }
}

#[test]
fn negative_zero_equals_zero_and_single_precision_compares_as_f32() {
    let mut reference = reference_json(&ours());
    *at(&mut reference, "/zero") = json!(-0.0_f64);
    *at(&mut reference, "/ratio") = json!(0.1_f64);
    assert_eq!(
        compared(&reference),
        Compared::Equal,
        "-0.0 and the short text of an f32"
    );
}

pub(crate) fn doc_with(field: Node) -> RecordDoc {
    RecordDoc {
        guid: "g".to_owned(),
        name: "n".to_owned(),
        tag: None,
        root: Node::Struct {
            type_name: "T".to_owned(),
            fields: vec![("f".to_owned(), field)],
        },
    }
}

#[test]
fn a_difference_next_to_an_allowed_one_still_fails() {
    let mut ours = doc_with(Node::Array(vec![Node::WeakPointer, Node::Int(1)]));
    let mut theirs = doc_with(Node::Array(vec![
        Node::Null,
        Node::Number(JsonNumber::Signed(2)),
    ]));
    assert!(matches!(
        compare_records(&ours, &theirs),
        Compared::Different(_)
    ));
    ours.tag = Some("a".to_owned());
    theirs.tag = Some("b".to_owned());
    assert!(matches!(
        compare_records(&ours, &theirs),
        Compared::Different(_)
    ));
}

#[test]
fn a_reference_is_matched_by_path_when_the_reference_states_no_guid() {
    let ours = doc_with(link("x", "libs/a/b"));
    let same = doc_with(link("", "file://./../../libs/a/b.json"));
    let other = doc_with(link("", "file://./../../libs/a/c.json"));
    let cased = doc_with(link("", "file://./../../libs/a/B.json"));
    assert_eq!(compare_records(&ours, &same), Compared::Equal);
    for different in [other, cased] {
        assert!(matches!(
            compare_records(&ours, &different),
            Compared::Different(_)
        ));
    }
}

#[test]
fn guids_and_field_names_are_compared_case_sensitively() {
    let ours = doc_with(link("ab", "libs/a"));
    let upper = doc_with(link("AB", "libs/a"));
    assert!(matches!(
        compare_records(&ours, &upper),
        Compared::Different(_)
    ));
    let mut renamed = doc_with(Node::Null);
    if let Node::Struct { fields, .. } = &mut renamed.root
        && let Some(field) = fields.first_mut()
    {
        "F".clone_into(&mut field.0);
    }
    assert!(matches!(
        compare_records(&doc_with(Node::Null), &renamed),
        Compared::Different(_)
    ));
}

pub(crate) fn link(guid: &str, path: &str) -> Node {
    Node::Reference {
        guid: guid.to_owned(),
        path: path.to_owned(),
        embedded: false,
    }
}
