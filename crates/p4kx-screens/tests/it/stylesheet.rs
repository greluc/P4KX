// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The `stylesheetOverride` of a canvas widget: a style laid over the canvas it holds.

use p4kx_dataforge::DataForge;
use p4kx_screens::ScreenComposer;
use p4kx_testkit::{
    TestCanvas, TestCondition, TestDim, TestModifier, TestStyle, TestStyleEntry, TestValue,
    TestWidget,
};

use crate::support::{
    BLUE, GREEN, RED, SCREEN, TestResult, blue_at, database, empty_at, green_at, guid_text, stage,
};

const INNER: u128 = SCREEN + 0x100;
const GREEN_STYLE: u128 = SCREEN + 0x200;
const BLUE_STYLE: u128 = SCREEN + 0x201;

fn paint(name: &str, colour: p4kx_testkit::TestColour) -> TestStyleEntry {
    TestStyleEntry::new(name)
        .when(TestCondition::Type("Base"))
        .set(TestModifier::EnableBackground(true))
        .set(TestModifier::Background(colour))
}

fn inner() -> TestCanvas {
    TestCanvas::new(INNER, "inner", 10.0, 10.0)
        .widget(TestWidget::display("content").size(TestDim::Fixed(10.0), TestDim::Fixed(10.0)))
}

fn holder(name: &str, style: Option<u128>) -> TestWidget {
    let holder = TestWidget::new("BuildingBlocks_WidgetCanvas", name)
        .parent("row")
        .size(TestDim::Fixed(10.0), TestDim::Fixed(10.0))
        .with("canvas", TestValue::Reference(INNER));
    match style {
        Some(guid) => holder.with("stylesheetOverride", TestValue::Reference(guid)),
        None => holder,
    }
}

fn outer() -> TestCanvas {
    TestCanvas::new(SCREEN, "outer", 40.0, 10.0)
        .widget(TestWidget::display("row").flex("Row", "Start", "Start", 0.0))
        .widget(holder("plain", None))
        .widget(holder("green", Some(GREEN_STYLE)))
        .widget(holder("blue", Some(BLUE_STYLE)))
        .widget(holder("again", Some(GREEN_STYLE)))
}

fn render_outer() -> Result<p4kx_gfx::RgbaImage, Box<dyn std::error::Error>> {
    let bytes = database([
        outer().record(),
        inner().record(),
        TestStyle::new(GREEN_STYLE, "s_green", Vec::new())
            .entry(paint("green", GREEN))
            .record(),
        TestStyle::new(BLUE_STYLE, "s_blue", Vec::new())
            .entry(paint("blue", BLUE))
            .entry(paint("red then blue", RED))
            .entry(paint("blue again", BLUE))
            .record(),
    ]);
    let forge = DataForge::open(&bytes)?;
    let composed = ScreenComposer::new(&forge).compose(&guid_text(SCREEN), None)?;
    Ok(composed.rasterize(&stage())?.image)
}

#[test]
fn a_stylesheet_override_styles_the_canvas_of_its_widget_only() -> TestResult {
    let image = render_outer()?;
    assert!(empty_at(&image, 5, 5), "the holder without an override");
    assert!(green_at(&image, 15, 5), "the first override");
    assert!(
        blue_at(&image, 25, 5),
        "the second override, the last entry wins"
    );
    assert!(green_at(&image, 35, 5), "the same override again");
    Ok(())
}

#[test]
fn an_override_that_is_no_record_changes_nothing() -> TestResult {
    let bytes = database([
        outer().record(),
        inner().record(),
        TestStyle::new(GREEN_STYLE, "s_green", Vec::new())
            .entry(paint("green", GREEN))
            .record(),
    ]);
    let forge = DataForge::open(&bytes)?;
    let image = ScreenComposer::new(&forge)
        .compose(&guid_text(SCREEN), None)?
        .rasterize(&stage())?
        .image;
    assert!(empty_at(&image, 5, 5));
    assert!(green_at(&image, 15, 5));
    assert!(empty_at(&image, 25, 5), "the missing style adds nothing");
    Ok(())
}
