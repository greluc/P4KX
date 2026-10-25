// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The `Child` and `Count` selectors, the `breakConditions` of `Ancestor`, and the style
//! modifiers that change the layout.

use p4kx_screens::Limits;
use p4kx_testkit::{
    TestCanvas, TestCondition, TestDim, TestModifier, TestStyleEntry, TestWidget, clone_modifiers,
    tag_record,
};

use crate::support::{
    BLUE, GREEN, RED, SCREEN, TestResult, blue_at, compose_with, empty_at, green_at, red_at,
    render_with,
};

const LIST_TAG: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0101;
const LEAF_TAG: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0102;
const MARK_TAG: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0103;

fn tags() -> Vec<p4kx_testkit::TestRecord> {
    vec![
        tag_record(LIST_TAG, "List"),
        tag_record(LEAF_TAG, "Leaf"),
        tag_record(MARK_TAG, "Mark"),
    ]
}

fn paint(name: &str, colour: p4kx_testkit::TestColour) -> TestStyleEntry {
    TestStyleEntry::new(name)
        .set(TestModifier::EnableBackground(true))
        .set(TestModifier::Background(colour))
}

const fn square(widget: TestWidget) -> TestWidget {
    widget.size(TestDim::Fixed(10.0), TestDim::Fixed(10.0))
}

fn canvas(width: f32, height: f32) -> TestCanvas {
    TestCanvas::new(SCREEN, "selectors", width, height)
}

fn rows() -> TestCanvas {
    canvas(40.0, 10.0)
        .template(square(TestWidget::display("row")))
        .static_array("rows", 4)
        .widget(TestWidget::list("list", "row", "rows").flex("Row", "Start", "Start", 0.0))
}

#[test]
fn count_selects_the_first_and_every_nth_sibling_from_an_offset() -> TestResult {
    let even = paint("even", RED)
        .when(TestCondition::Type("Base"))
        .when(TestCondition::Count("Every", 2, 0));
    let second = paint("second", GREEN)
        .when(TestCondition::Type("Base"))
        .when(TestCondition::Count("First", 1, 1));
    let image = render_with(&rows().embedded(even).embedded(second), Vec::new())?;
    assert!(red_at(&image, 5, 5), "row 0 is every second row");
    assert!(
        green_at(&image, 15, 5),
        "row 1 is the first row from offset 1"
    );
    assert!(red_at(&image, 25, 5), "row 2 is every second row");
    assert!(empty_at(&image, 35, 5), "row 3 is neither");
    Ok(())
}

#[test]
fn count_first_takes_as_many_siblings_as_it_says() -> TestResult {
    let entry = paint("first two", BLUE)
        .when(TestCondition::Type("Base"))
        .when(TestCondition::Count("First", 2, 0));
    let image = render_with(&rows().embedded(entry), Vec::new())?;
    assert!(blue_at(&image, 5, 5));
    assert!(blue_at(&image, 15, 5));
    assert!(empty_at(&image, 25, 5));
    assert!(empty_at(&image, 35, 5));
    Ok(())
}

#[test]
fn counts_that_make_no_sense_select_nothing_and_cannot_overflow() -> TestResult {
    let negative = paint("negative", RED)
        .when(TestCondition::Type("Base"))
        .when(TestCondition::Count("First", -1, 0));
    let huge = paint("huge", GREEN)
        .when(TestCondition::Type("Base"))
        .when(TestCondition::Count("Every", i32::MAX, i32::MAX));
    let zero = paint("zero", BLUE)
        .when(TestCondition::Type("Base"))
        .when(TestCondition::Count("Every", 0, 0));
    let image = render_with(
        &rows().embedded(negative).embedded(huge).embedded(zero),
        Vec::new(),
    )?;
    for x in [5, 15, 25, 35] {
        assert!(empty_at(&image, x, 5), "no row at x = {x} is selected");
    }
    Ok(())
}

#[test]
fn the_position_of_a_widget_is_counted_among_all_its_siblings() -> TestResult {
    let entry = paint("last", RED)
        .when(TestCondition::Type("Base"))
        .when(TestCondition::Count("First", 1, 2));
    let canvas = canvas(40.0, 10.0)
        .embedded(entry)
        .widget(TestWidget::display("row").flex("Row", "Start", "Start", 0.0))
        .widget(square(TestWidget::display("a").parent("row")))
        .widget(square(TestWidget::display("b").parent("row")).hidden())
        .widget(square(TestWidget::display("c").parent("row")));
    let image = render_with(&canvas, Vec::new())?;
    assert!(
        red_at(&image, 15, 5),
        "the third sibling is the visible second one"
    );
    assert!(empty_at(&image, 5, 5));
    Ok(())
}

#[test]
fn a_child_selector_looks_at_the_children_of_the_widget() -> TestResult {
    let entry = paint("holders", RED)
        .when(TestCondition::Type("Card"))
        .when(TestCondition::Child(vec![TestCondition::Tag(MARK_TAG)]));
    let card =
        |name: &str| square(TestWidget::new("BuildingBlocks_WidgetCard", name)).parent("row");
    let canvas = canvas(40.0, 10.0)
        .embedded(entry)
        .widget(TestWidget::display("row").flex("Row", "Start", "Start", 0.0))
        .widget(card("with"))
        .widget(TestWidget::display("marked").parent("with").tag(MARK_TAG))
        .widget(card("without"))
        .widget(TestWidget::display("plain").parent("without"))
        .widget(card("empty"));
    let image = render_with(&canvas, tags())?;
    assert!(red_at(&image, 5, 5), "the card with a marked child");
    assert!(empty_at(&image, 15, 5), "the card with an unmarked child");
    assert!(empty_at(&image, 25, 5), "the card without children");
    Ok(())
}

#[test]
fn a_child_selector_with_a_parent_inside_is_reported_and_does_not_apply() -> TestResult {
    let entry = paint("odd", RED)
        .when(TestCondition::Type("Base"))
        .when(TestCondition::Child(vec![TestCondition::Parent(vec![
            TestCondition::Tag(MARK_TAG),
        ])]));
    let canvas = canvas(10.0, 10.0)
        .embedded(entry)
        .widget(TestWidget::display("root").tag(MARK_TAG))
        .widget(TestWidget::display("kid").parent("root"));
    let composed = compose_with(&canvas, tags(), Limits::DEFAULT)?;
    let note = composed
        .report()
        .unsupported
        .iter()
        .find(|entry| entry.element == "StyleEntry")
        .ok_or("the entry is not reported")?;
    assert!(note.reason.contains("Child"), "{}", note.reason);
    assert_eq!(composed.report().shapes, 0);
    Ok(())
}

fn first_of_the_list() -> TestCanvas {
    canvas(40.0, 10.0)
        .widget(
            TestWidget::display("list")
                .tag(LIST_TAG)
                .flex("Row", "Start", "Start", 0.0),
        )
        .widget(
            square(TestWidget::display("item0").parent("list"))
                .size(TestDim::Fixed(20.0), TestDim::Fixed(10.0)),
        )
        .widget(TestWidget::display("leaf0").parent("item0").tag(LEAF_TAG))
        .widget(
            square(TestWidget::display("item1").parent("list"))
                .size(TestDim::Fixed(20.0), TestDim::Fixed(10.0)),
        )
        .widget(TestWidget::display("leaf1").parent("item1").tag(LEAF_TAG))
}

#[test]
fn an_ancestor_selector_stops_at_the_widget_its_break_conditions_name() -> TestResult {
    let entry = paint("first item", RED)
        .when(TestCondition::Tag(LEAF_TAG))
        .when(TestCondition::AncestorUntil(
            vec![TestCondition::Count("First", 1, 0)],
            vec![TestCondition::Tag(LIST_TAG)],
        ));
    let image = render_with(&first_of_the_list().embedded(entry), tags())?;
    assert!(red_at(&image, 10, 5), "the leaf of the first item");
    assert!(
        empty_at(&image, 30, 5),
        "the leaf of the second item: the list itself is a first sibling but is past the break"
    );
    Ok(())
}

#[test]
fn without_break_conditions_an_ancestor_selector_looks_all_the_way_up() -> TestResult {
    let entry = paint("any first", RED)
        .when(TestCondition::Tag(LEAF_TAG))
        .when(TestCondition::Ancestor(vec![TestCondition::Count(
            "First", 1, 0,
        )]));
    let image = render_with(&first_of_the_list().embedded(entry), tags())?;
    assert!(red_at(&image, 10, 5));
    assert!(red_at(&image, 30, 5), "the list is a first root");
    Ok(())
}

#[test]
fn a_style_margin_moves_the_siblings_after_it() -> TestResult {
    let entry = TestStyleEntry::new("margin")
        .when(TestCondition::Tag(MARK_TAG))
        .set(TestModifier::Number("MarginLeft", 10.0));
    let canvas = canvas(40.0, 10.0)
        .embedded(entry)
        .widget(TestWidget::display("row").flex("Row", "Start", "Start", 0.0))
        .widget(square(
            TestWidget::display("a")
                .parent("row")
                .tag(MARK_TAG)
                .background(RED),
        ))
        .widget(square(
            TestWidget::display("b").parent("row").background(GREEN),
        ));
    let image = render_with(&canvas, tags())?;
    assert!(empty_at(&image, 5, 5), "the margin");
    assert!(red_at(&image, 15, 5), "the widget behind its margin");
    assert!(green_at(&image, 25, 5), "the sibling is pushed on");
    Ok(())
}

#[test]
fn a_style_can_read_a_size_as_a_share_of_the_parent() -> TestResult {
    let entry = TestStyleEntry::new("half")
        .when(TestCondition::Tag(MARK_TAG))
        .set(TestModifier::Number("SizeX", 0.5))
        .set(TestModifier::Enumerated("WidthBehavior", "Percent"));
    let canvas = canvas(40.0, 10.0).embedded(entry).widget(square(
        TestWidget::display("bar").tag(MARK_TAG).background(RED),
    ));
    let image = render_with(&canvas, tags())?;
    assert!(red_at(&image, 15, 5), "half of the canvas is 20 pixels");
    assert!(red_at(&image, 19, 5));
    assert!(empty_at(&image, 25, 5));
    Ok(())
}

#[test]
fn a_style_can_make_a_flex_item_grow() -> TestResult {
    let entry = TestStyleEntry::new("grow")
        .when(TestCondition::Tag(MARK_TAG))
        .set(TestModifier::Number("FlexGrowProportion", 1.0));
    let canvas = canvas(40.0, 10.0)
        .embedded(entry)
        .widget(TestWidget::display("row").flex("Row", "Start", "Start", 0.0))
        .widget(square(
            TestWidget::display("a").parent("row").background(RED),
        ))
        .widget(square(
            TestWidget::display("b")
                .parent("row")
                .tag(MARK_TAG)
                .background(GREEN),
        ));
    let image = render_with(&canvas, tags())?;
    assert!(red_at(&image, 5, 5));
    assert!(green_at(&image, 15, 5));
    assert!(green_at(&image, 38, 5), "the second takes the free space");
    Ok(())
}

#[test]
fn a_style_can_turn_a_row_into_a_column() -> TestResult {
    let entry = TestStyleEntry::new("column")
        .when(TestCondition::Tag(LIST_TAG))
        .set(TestModifier::Enumerated("FlexDirection", "Column"));
    let canvas = canvas(20.0, 20.0)
        .embedded(entry)
        .widget(
            TestWidget::display("box")
                .tag(LIST_TAG)
                .flex("Row", "Start", "Start", 0.0),
        )
        .widget(square(
            TestWidget::display("a").parent("box").background(RED),
        ))
        .widget(square(
            TestWidget::display("b").parent("box").background(GREEN),
        ));
    let image = render_with(&canvas, tags())?;
    assert!(red_at(&image, 5, 5));
    assert!(green_at(&image, 5, 15), "the second is below the first");
    assert!(empty_at(&image, 15, 5));
    Ok(())
}

#[test]
fn style_numbers_that_are_not_finite_or_huge_cannot_break_the_layout() -> TestResult {
    let entry = TestStyleEntry::new("wild")
        .when(TestCondition::Tag(MARK_TAG))
        .set(TestModifier::Number("MarginLeft", f32::NAN))
        .set(TestModifier::Number("MarginTop", f32::INFINITY))
        .set(TestModifier::Number("FlexGrowProportion", f32::NAN))
        .set(TestModifier::Number("OrientationZOffset", f32::INFINITY))
        .set(TestModifier::Number("SizeX", 1.0e30));
    let canvas = canvas(40.0, 10.0).embedded(entry).widget(square(
        TestWidget::display("wild").tag(MARK_TAG).background(RED),
    ));
    let composed = compose_with(&canvas, tags(), Limits::DEFAULT)?;
    assert!(!composed.report().truncated);
    Ok(())
}

#[test]
fn the_modifiers_of_clones_in_force_are_capped() -> TestResult {
    let clone = || {
        TestWidget::clone_of("copy", "tmpl").with(
            "modifiers",
            clone_modifiers(&[
                TestModifier::Background(BLUE),
                TestModifier::EnableBackground(true),
            ]),
        )
    };
    let build = || {
        canvas(10.0, 10.0)
            .template(square(TestWidget::display("tmpl")))
            .widget(clone())
    };
    let free = compose_with(&build(), Vec::new(), Limits::DEFAULT)?;
    assert!(!free.report().truncated);
    assert!(blue_at(
        &free.rasterize(&crate::support::stage())?.image,
        5,
        5
    ));

    let per_clone = Limits {
        overrides: [32, 1],
        ..Limits::DEFAULT
    };
    let capped = compose_with(&build(), Vec::new(), per_clone)?;
    assert!(capped.report().truncated, "the second modifier is dropped");
    assert!(
        empty_at(&capped.rasterize(&crate::support::stage())?.image, 5, 5),
        "the background is not enabled"
    );

    let in_force = Limits {
        overrides: [0, 256],
        ..Limits::DEFAULT
    };
    let none = compose_with(&build(), Vec::new(), in_force)?;
    assert!(none.report().truncated, "no clone may be in force");
    Ok(())
}
