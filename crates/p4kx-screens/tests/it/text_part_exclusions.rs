// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! What the text of a text field does not take from the entries (ADR-0296): the entry that would
//! turn it green is tried first and the field's own entry, red, comes last, so green shows only
//! where the text part wrongly applied that entry.

use p4kx_testkit::{TestCanvas, TestCondition, TestModifier, TestStyleEntry, TestWidget};

use crate::support::{SCREEN, TestResult, near, pixel};
use crate::text_part::{BELOW, BLUE, GREEN, PART, field, field_entry, render};

const TEXT_AT: (u32, u32) = (32, 4);

fn below() -> TestCondition {
    TestCondition::Parent(vec![TestCondition::Tag(BELOW)])
}

fn green() -> TestStyleEntry {
    TestStyleEntry::new("green").set(TestModifier::Fill(GREEN))
}

fn canvas(first: TestStyleEntry, tagged: TestWidget) -> TestCanvas {
    TestCanvas::new(SCREEN, "text_part_exclusions", 60.0, 20.0)
        .embedded(first)
        .embedded(field_entry())
        .widget(TestWidget::display("root"))
        .widget(tagged)
}

fn tagged() -> TestWidget {
    field("tagged", 30.0).tag(BELOW)
}

fn colour_of_text(canvas: &TestCanvas) -> Result<[u8; 4], Box<dyn std::error::Error>> {
    Ok(pixel(&render(canvas)?, TEXT_AT.0, TEXT_AT.1))
}

fn assert_red(canvas: &TestCanvas, why: &str) -> TestResult {
    let found = colour_of_text(canvas)?;
    assert!(near(found, [255, 0, 0, 255], 2), "{why}: {found:?}");
    Ok(())
}

#[test]
fn a_parent_next_to_another_type_stays_with_the_field() -> TestResult {
    let entry = green().when(TestCondition::Type("Card")).when(below());
    assert_red(&canvas(entry, tagged()), "a card entry")
}

#[test]
fn a_selector_by_tag_alone_stays_with_the_field() -> TestResult {
    let entry = green().when(TestCondition::Tag(PART));
    assert_red(&canvas(entry, tagged().tag(PART)), "a tag entry")
}

#[test]
fn a_selector_by_count_alone_stays_with_the_field() -> TestResult {
    let entry = green().when(TestCondition::Count("Every", 1, 0));
    assert_red(&canvas(entry, tagged()), "a count entry")
}

#[test]
fn the_inline_style_of_the_field_stays_with_the_field() -> TestResult {
    let by_parent = TestStyleEntry::new("by_parent")
        .when(below())
        .set(TestModifier::Fill(BLUE));
    let inline = green().when(below());
    let widget = tagged().inline_style(&inline);
    let found = colour_of_text(&canvas(by_parent, widget))?;
    assert!(
        near(found, [0, 0, 255, 255], 2),
        "the entry of the sheet colours the text, the inline one does not: {found:?}"
    );
    Ok(())
}

#[test]
fn a_parent_inside_an_all_of_applies() -> TestResult {
    let entry = green().when(TestCondition::AllOf(vec![below()]));
    let found = colour_of_text(&canvas(entry, tagged()))?;
    assert!(near(found, [0, 255, 0, 255], 2), "{found:?}");
    Ok(())
}

#[test]
fn a_parent_with_a_named_type_inside_an_all_of_stays_with_the_field() -> TestResult {
    let entry = green().when(TestCondition::AllOf(vec![
        below(),
        TestCondition::Type("Card"),
    ]));
    assert_red(&canvas(entry, tagged()), "an all-of with a card")
}

#[test]
fn only_the_list_that_selects_the_text_counts_in_an_entry_with_several() -> TestResult {
    let entry = green()
        .when(TestCondition::Type("Text"))
        .when(TestCondition::Parent(vec![TestCondition::Tag(PART)]))
        .or_list(vec![TestCondition::Count("Every", 1, 0)]);
    assert_red(&canvas(entry, tagged()), "a count list next to a text list")
}

#[test]
fn a_parent_entry_gives_the_text_only_what_a_text_draws() -> TestResult {
    let entry = green()
        .when(below())
        .set(TestModifier::Alpha(0.0))
        .set(TestModifier::Active(false))
        .set(TestModifier::Number("SizeX", 1.0));
    let found = colour_of_text(&canvas(entry, tagged()))?;
    assert!(
        near(found, [0, 255, 0, 255], 2),
        "the colour applies, the alpha, activity and size do not: {found:?}"
    );
    Ok(())
}
