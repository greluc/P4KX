// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::condition::{Condition, CountKind, selects_text};

fn parent() -> Condition {
    Condition::Parent(vec![Condition::AnyOfTag(Vec::new())])
}

fn kind(name: &str) -> Condition {
    Condition::Type(name.to_owned())
}

#[test]
fn a_selector_by_the_parent_without_a_type_selects_the_text() {
    assert!(selects_text(&[parent()], "Text"));
}

#[test]
fn a_selector_that_requires_the_type_text_selects_it_with_or_without_a_parent() {
    assert!(selects_text(&[kind("Text")], "Text"));
    assert!(selects_text(&[kind("Text"), parent()], "Text"));
}

#[test]
fn a_parent_next_to_another_type_does_not() {
    assert!(!selects_text(&[parent(), kind("Card")], "Text"));
}

#[test]
fn tags_and_counts_alone_do_not() {
    assert!(!selects_text(&[Condition::AnyOfTag(Vec::new())], "Text"));
    let count = Condition::Count {
        kind: CountKind::Every,
        count: 1,
        offset: 0,
    };
    assert!(!selects_text(&[count], "Text"));
    assert!(!selects_text(&[], "Text"));
}

#[test]
fn a_parent_inside_an_all_of_selects_the_text() {
    assert!(selects_text(&[Condition::AllOf(vec![parent()])], "Text"));
}

#[test]
fn a_parent_with_a_named_type_inside_an_all_of_does_not() {
    let inner = vec![parent(), kind("Card")];
    assert!(!selects_text(&[Condition::AllOf(inner)], "Text"));
}

#[test]
fn a_type_asked_of_the_parent_is_not_a_type_of_the_widget() {
    let above = Condition::Parent(vec![kind("Card")]);
    assert!(selects_text(&[above], "Text"));
}
