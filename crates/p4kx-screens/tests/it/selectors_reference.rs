// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A synthetic canvas that uses the selectors, the layout modifiers and the state at rest
//! together, against a stored reference image.
//!
//! Set `P4KX_UPDATE_REFERENCE` to write the reference again after a deliberate change.

use p4kx_testkit::{
    TestCanvas, TestColour, TestCondition, TestDim, TestModifier, TestOp, TestStyleEntry,
    TestWidget, tag_record,
};

use crate::support::{BLUE, GREEN, RED, SCREEN, TestResult, matches_reference, render_with};

const REFERENCE: &str = "tests/it/reference/cascade_sample.png";
const YELLOW: TestColour = TestColour::Solid([255, 255, 0, 255]);
const GREY: TestColour = TestColour::Solid([50, 50, 60, 255]);
const LIGHT: TestColour = TestColour::Solid([110, 110, 130, 255]);
const MARK: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0201;
const SHIFT: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0202;
const HALF: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0203;
const TILE: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0204;

fn paint(name: &str, colour: TestColour) -> TestStyleEntry {
    TestStyleEntry::new(name)
        .set(TestModifier::EnableBackground(true))
        .set(TestModifier::Background(colour))
}

fn line(widget: TestWidget) -> TestWidget {
    widget
        .parent("page")
        .size(TestDim::Percent(1.0), TestDim::Fixed(14.0))
        .flex("Row", "Start", "Start", 4.0)
}

fn cell(name: &str, parent: &str) -> TestWidget {
    TestWidget::display(name)
        .parent(parent)
        .size(TestDim::Fixed(14.0), TestDim::Fixed(14.0))
        .background(LIGHT)
}

fn sample() -> TestCanvas {
    widgets(styles(
        TestCanvas::new(SCREEN, "sample", 160.0, 100.0)
            .template(
                TestWidget::display("tile")
                    .tag(TILE)
                    .size(TestDim::Fixed(20.0), TestDim::Fixed(14.0))
                    .background(LIGHT),
            )
            .static_array("tiles", 6),
    ))
}

fn styles(canvas: TestCanvas) -> TestCanvas {
    canvas
        .embedded(
            paint("every second", RED)
                .when(TestCondition::Tag(TILE))
                .when(TestCondition::Count("Every", 2, 0)),
        )
        .embedded(
            paint("the sixth", YELLOW)
                .when(TestCondition::Tag(TILE))
                .when(TestCondition::Count("First", 1, 5)),
        )
        .embedded(
            paint("holders", GREEN)
                .when(TestCondition::Type("Card"))
                .when(TestCondition::Child(vec![TestCondition::Tag(MARK)])),
        )
        .embedded(
            TestStyleEntry::new("half")
                .when(TestCondition::Tag(HALF))
                .set(TestModifier::Number("SizeX", 0.5))
                .set(TestModifier::Enumerated("WidthBehavior", "Percent")),
        )
        .embedded(
            TestStyleEntry::new("shift")
                .when(TestCondition::Tag(SHIFT))
                .set(TestModifier::Number("MarginLeft", 12.0)),
        )
}

fn widgets(canvas: TestCanvas) -> TestCanvas {
    let card = |name: &str| {
        TestWidget::new("BuildingBlocks_WidgetCard", name)
            .parent("cards")
            .size(TestDim::Fixed(20.0), TestDim::Fixed(14.0))
            .background(BLUE)
    };
    canvas
        .operation("flag", TestOp::BoolVar)
        .operation("fade", TestOp::NumFromBool("flag", 1.0, 0.4))
        .widget(TestWidget::display("backdrop").background(GREY))
        .widget(
            TestWidget::display("page")
                .padding([4.0; 4])
                .flex("Column", "Start", "Start", 4.0),
        )
        .widget(line(TestWidget::list("tiles", "tile", "tiles")))
        .widget(line(TestWidget::display("cards")))
        .widget(card("first"))
        .widget(TestWidget::display("mark").parent("first").tag(MARK))
        .widget(card("second"))
        .widget(TestWidget::display("plain").parent("second"))
        .widget(
            TestWidget::display("bar")
                .parent("page")
                .size(TestDim::Fixed(10.0), TestDim::Fixed(8.0))
                .tag(HALF)
                .background(YELLOW),
        )
        .widget(line(TestWidget::display("moved")))
        .widget(cell("a", "moved"))
        .widget(cell("b", "moved").tag(SHIFT))
        .widget(cell("c", "moved"))
        .widget(line(TestWidget::display("state")))
        .widget(cell("gone", "state").background(RED))
        .widget(cell("faded", "state").background(RED))
        .widget(cell("hidden", "state").background(RED))
        .widget(cell("plain state", "state").background(GREEN))
        .operation(
            "bind_gone",
            TestOp::BoolField("gone", "Instantiated", "flag"),
        )
        .operation("bind_fade", TestOp::NumField("faded", "Alpha", "fade"))
        .operation(
            "bind_hidden",
            TestOp::BoolField("hidden", "IsActive", "flag"),
        )
}

#[test]
fn the_cascade_sample_matches_its_reference_image() -> TestResult {
    let tags = vec![
        tag_record(MARK, "Mark"),
        tag_record(SHIFT, "Shift"),
        tag_record(HALF, "Half"),
        tag_record(TILE, "Tile"),
    ];
    let image = render_with(&sample(), tags)?;
    matches_reference(REFERENCE, &image, 3, 8)
}
