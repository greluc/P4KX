// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Mutation self-test of the `StarBreaker` allow-list: each entry applies to its case only, and
//! every allowed value is counted.

use super::mutation_value::{doc_with, link};
use super::tally::Compared;
use super::value_tree::{JsonNumber, Node};
use super::value_tree_compare::{
    DANGLING_NULL, LENIENT_BOOL, NON_FINITE_NULL, NON_FINITE_TEXT, WEAK_NULL, compare_records,
};

fn text(value: &str) -> Node {
    Node::Text(value.to_owned())
}

const fn signed(value: i64) -> Node {
    Node::Number(JsonNumber::Signed(value))
}

fn error() -> Node {
    Node::Error("the array lies outside the pool".to_owned())
}

fn unmapped() -> Node {
    Node::Unmapped("Value::Future".to_owned())
}

fn outcome(ours: &Node, theirs: &Node) -> Compared {
    compare_records(&doc_with(ours.clone()), &doc_with(theirs.clone()))
}

fn assert_allowed(ours: &Node, theirs: &Node, reason: &'static str) {
    assert_eq!(
        outcome(ours, theirs),
        Compared::Allowed(vec![reason]),
        "{ours:?} against {theirs:?}"
    );
}

fn assert_different(ours: &Node, theirs: &Node) {
    let compared = outcome(ours, theirs);
    assert!(
        matches!(compared, Compared::Different(_)),
        "{ours:?} against {theirs:?} is {compared:?}"
    );
}

#[test]
fn a_weak_pointer_is_allowed_against_null_only() {
    assert_allowed(&Node::WeakPointer, &Node::Null, WEAK_NULL);
    for theirs in [
        text("value"),
        Node::Bool(false),
        signed(0),
        link("g", "p"),
        Node::Array(Vec::new()),
    ] {
        assert_different(&Node::WeakPointer, &theirs);
    }
}

#[test]
fn only_a_dangling_reference_is_allowed_against_null() {
    assert_allowed(&Node::Dangling, &Node::Null, DANGLING_NULL);
    for ours in [error(), unmapped(), Node::InvalidBool] {
        assert_different(&ours, &Node::Null);
    }
    for theirs in [text("value"), Node::Bool(true), signed(0), link("g", "p")] {
        assert_different(&Node::Dangling, &theirs);
    }
}

#[test]
fn only_an_invalid_bool_is_allowed_against_true() {
    assert_allowed(&Node::InvalidBool, &Node::Bool(true), LENIENT_BOOL);
    assert_different(&Node::InvalidBool, &Node::Bool(false));
    assert_different(&Node::InvalidBool, &Node::Null);
    assert_different(&Node::InvalidBool, &signed(1));
    for ours in [error(), unmapped(), Node::Dangling, Node::WeakPointer] {
        assert_different(&ours, &Node::Bool(true));
        assert_different(&ours, &Node::Bool(false));
    }
}

#[test]
fn a_non_finite_number_is_allowed_against_null_or_its_own_text() {
    assert_allowed(&Node::Single(f32::NAN), &Node::Null, NON_FINITE_NULL);
    assert_allowed(&Node::Double(f64::INFINITY), &Node::Null, NON_FINITE_NULL);
    assert_allowed(&Node::Single(f32::NAN), &text("NaN"), NON_FINITE_TEXT);
    assert_allowed(&Node::Single(f32::INFINITY), &text("INF"), NON_FINITE_TEXT);
    assert_allowed(
        &Node::Double(f64::NEG_INFINITY),
        &text("-INF"),
        NON_FINITE_TEXT,
    );
}

#[test]
fn a_finite_number_or_another_text_is_no_non_finite_allowance() {
    for finite in [Node::Single(1.5), Node::Double(-2.5), Node::Single(0.0)] {
        assert_different(&finite, &Node::Null);
        for word in ["NaN", "INF", "-INF"] {
            assert_different(&finite, &text(word));
        }
    }
    assert_different(&Node::Single(f32::NAN), &text("INF"));
    assert_different(&Node::Double(f64::INFINITY), &text("-INF"));
    assert_different(&Node::Double(f64::NEG_INFINITY), &text("INF"));
    assert_different(&Node::Double(f64::INFINITY), &text("Infinity"));
    assert_different(&Node::Double(f64::NAN), &signed(0));
    assert_different(&Node::Int(0), &Node::Null);
}

#[test]
fn every_allowed_value_is_counted_under_its_reason() {
    let ours = doc_with(Node::Array(vec![
        Node::WeakPointer,
        Node::Dangling,
        Node::WeakPointer,
        Node::Single(f32::NAN),
    ]));
    let theirs = doc_with(Node::Array(vec![
        Node::Null,
        Node::Null,
        Node::Null,
        text("NaN"),
    ]));
    assert_eq!(
        compare_records(&ours, &theirs),
        Compared::Allowed(vec![WEAK_NULL, DANGLING_NULL, WEAK_NULL, NON_FINITE_TEXT])
    );
}
