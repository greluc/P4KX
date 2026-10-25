// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Selectors of a nested canvas that look at the place and the children of the widgets above it,
//! under an outer canvas whose own styles use none.

use p4kx_dataforge::DataForge;
use p4kx_screens::ScreenComposer;
use p4kx_testkit::{
    TestCanvas, TestCondition, TestDim, TestModifier, TestStyleEntry, TestValue, TestWidget,
};

use crate::support::{
    BLUE, GREEN, SCREEN, TestResult, blue_at, database, empty_at, green_at, guid_text, stage,
};

const INNER: u128 = SCREEN + 0x100;

fn paint(colour: p4kx_testkit::TestColour, condition: TestCondition) -> TestStyleEntry {
    TestStyleEntry::new("paint")
        .when(TestCondition::Type("Base"))
        .when(condition)
        .set(TestModifier::EnableBackground(true))
        .set(TestModifier::Background(colour))
}

fn inner(entry: TestStyleEntry) -> TestCanvas {
    TestCanvas::new(INNER, "inner", 10.0, 10.0)
        .embedded(entry)
        .widget(TestWidget::display("content").size(TestDim::Fixed(10.0), TestDim::Fixed(10.0)))
}

fn holder(name: &str) -> TestWidget {
    TestWidget::new("BuildingBlocks_WidgetCanvas", name)
        .parent("row")
        .size(TestDim::Fixed(10.0), TestDim::Fixed(10.0))
        .with("canvas", TestValue::Reference(INNER))
}

fn render(
    outer: &TestCanvas,
    inner: &TestCanvas,
) -> Result<p4kx_gfx::RgbaImage, Box<dyn std::error::Error>> {
    let bytes = database([outer.record(), inner.record()]);
    let forge = DataForge::open(&bytes)?;
    let composed = ScreenComposer::new(&forge).compose(&guid_text(SCREEN), None)?;
    Ok(composed.rasterize(&stage())?.image)
}

fn row() -> TestCanvas {
    TestCanvas::new(SCREEN, "outer", 30.0, 10.0)
        .widget(TestWidget::display("row").flex("Row", "Start", "Start", 0.0))
}

#[test]
fn a_nested_ancestor_count_sees_the_place_of_the_widgets_above_it() -> TestResult {
    let outer = row()
        .widget(holder("first"))
        .widget(holder("second"))
        .widget(holder("third"));
    let entry = paint(
        GREEN,
        TestCondition::Ancestor(vec![
            TestCondition::Type("Canvas"),
            TestCondition::Count("First", 1, 1),
        ]),
    );
    let image = render(&outer, &inner(entry))?;
    assert!(empty_at(&image, 5, 5), "the first holder is not at place 1");
    assert!(green_at(&image, 15, 5), "the second holder is");
    assert!(empty_at(&image, 25, 5), "the third holder is not");
    Ok(())
}

#[test]
fn a_nested_parent_child_sees_the_children_of_the_widget_above_it() -> TestResult {
    let outer = row()
        .widget(holder("with a child"))
        .widget(TestWidget::display("kid").parent("with a child"))
        .widget(holder("alone"));
    let entry = paint(
        BLUE,
        TestCondition::Parent(vec![TestCondition::Child(vec![TestCondition::Type(
            "Base",
        )])]),
    );
    let image = render(&outer, &inner(entry))?;
    assert!(blue_at(&image, 5, 5), "the holder that has a child");
    assert!(empty_at(&image, 15, 5), "the holder that has none");
    Ok(())
}
