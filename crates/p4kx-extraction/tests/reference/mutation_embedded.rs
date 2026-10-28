// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Mutation self-test of the comparison of embedded Records: the content `StarBreaker` writes in
//! place of a link is compared with the Record P4KX holds, and every change to it is detected.

#![expect(
    clippy::expect_used,
    reason = "the helpers mutate synthetic data; a value that is not there fails the test"
)]

use serde_json::{Value, json};

use super::mutation_value::{doc_with, link};
use super::samples::{OTHER_GUID, SAMPLE_GUID, embedded_json, ours, reference_json, resolve};
use super::starbreaker_json::record;
use super::tally::Compared;
use super::value_tree::{Embedded, JsonNumber, Node, RecordDoc};
use super::value_tree_compare::{compare_counting, compare_records};

const LINK: &str = "/_RecordValue_/link";

/// The reference JSON of the synthetic Record with its link replaced by the embedded content of
/// the Record it links to.
fn embedding_reference() -> Value {
    let mut reference = reference_json(&ours());
    let other = resolve(OTHER_GUID).expect("the linked Record exists");
    *reference.pointer_mut(LINK).expect("the link field") = embedded_json(&other);
    reference
}

fn compared(reference: &Value) -> (Compared, u64) {
    let theirs = record(&reference.to_string()).expect("a Record");
    compare_counting(&ours(), &theirs, &mut resolve)
}

fn embedded_field<'v>(reference: &'v mut Value, key: &str) -> &'v mut Value {
    reference
        .pointer_mut(&format!("{LINK}/{key}"))
        .expect("the embedded Record has this field")
}

fn assert_different(reference: &Value, what: &str) {
    let (result, _) = compared(reference);
    assert!(
        matches!(result, Compared::Different(_)),
        "{what} must be detected, got {result:?}"
    );
}

#[test]
fn embedded_content_equal_to_the_record_is_equal_and_counted() {
    let (result, embedded) = compared(&embedding_reference());
    assert!(
        matches!(result, Compared::Equal | Compared::Allowed(_)),
        "{result:?}"
    );
    assert_eq!(embedded, 1, "one embedded Record");
}

#[test]
fn every_change_to_the_embedded_content_is_detected() {
    type Mutation = (&'static str, fn(&mut Value));
    let mutations: [Mutation; 8] = [
        ("integer plus one", |reference| {
            *embedded_field(reference, "count") = json!(1);
        }),
        ("boolean flipped", |reference| {
            *embedded_field(reference, "flag") = json!(true);
        }),
        ("text changed", |reference| {
            *embedded_field(reference, "label") = json!("changed");
        }),
        ("Record name changed", |reference| {
            *embedded_field(reference, "_RecordName_") = json!("Sample.Else");
        }),
        ("struct type changed", |reference| {
            *embedded_field(reference, "_Type_") = json!("Else");
        }),
        ("Record GUID changed", |reference| {
            *embedded_field(reference, "_RecordId_") = json!(SAMPLE_GUID);
        }),
        ("field deleted", |reference| {
            reference
                .pointer_mut(LINK)
                .and_then(Value::as_object_mut)
                .expect("the embedded Record")
                .remove("label")
                .expect("the field exists");
        }),
        ("field invented", |reference| {
            reference
                .pointer_mut(LINK)
                .and_then(Value::as_object_mut)
                .expect("the embedded Record")
                .insert("invented".to_owned(), json!(1));
        }),
    ];
    for (what, mutate) in mutations {
        let mut reference = embedding_reference();
        mutate(&mut reference);
        assert_different(&reference, what);
    }
}

#[test]
fn an_embedded_record_without_a_record_to_compare_with_is_a_difference() {
    let theirs = record(&embedding_reference().to_string()).expect("a Record");
    assert!(matches!(
        compare_records(&ours(), &theirs),
        Compared::Different(_)
    ));
}

fn named(guid: &str, name: &str, field: Node) -> RecordDoc {
    let mut doc = doc_with(field);
    guid.clone_into(&mut doc.guid);
    name.clone_into(&mut doc.name);
    doc
}

fn embedding(guid: &str, name: &str, field: Node) -> Node {
    Node::Reference {
        guid: guid.to_owned(),
        path: String::new(),
        embedded: Some(Box::new(Embedded {
            name: name.to_owned(),
            root: doc_with(field).root,
        })),
    }
}

fn resolve_chain(guid: &str) -> Option<RecordDoc> {
    match guid {
        "h" => Some(named("h", "nh", link("i", "p"))),
        "i" => Some(named("i", "ni", Node::Int(7))),
        _ => None,
    }
}

#[test]
fn embedded_records_inside_embedded_records_are_compared_and_counted() {
    let ours = doc_with(link("h", "p"));
    let theirs_with = |value: i64| {
        let inner = embedding("i", "ni", Node::Number(JsonNumber::Signed(value)));
        doc_with(embedding("h", "nh", inner))
    };
    assert_eq!(
        compare_counting(&ours, &theirs_with(7), &mut resolve_chain),
        (Compared::Equal, 2)
    );
    let (result, embedded) = compare_counting(&ours, &theirs_with(8), &mut resolve_chain);
    assert!(matches!(result, Compared::Different(_)), "{result:?}");
    assert_eq!(embedded, 2, "both were reached before the difference");
}

#[test]
fn an_embedded_record_is_still_compared_by_target() {
    let ours = doc_with(Node::Array(vec![link("h", "p"), link("h", "p")]));
    let embedded = embedding("h", "nh", link("i", "p"));
    let theirs = doc_with(Node::Array(vec![embedded.clone(), link("h", "p")]));
    assert_eq!(
        compare_counting(&ours, &theirs, &mut resolve_chain),
        (Compared::Equal, 1)
    );
    let other = doc_with(Node::Array(vec![embedded, link("g", "p")]));
    assert!(matches!(
        compare_counting(&ours, &other, &mut resolve_chain).0,
        Compared::Different(_)
    ));
}
