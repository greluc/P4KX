// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The canvas a canvas widget holds, chosen by a style entry or a binding (ADR-0277), and the
//! namespace of a variable setting.

use p4kx_dataforge::DataForge;
use p4kx_screens::{Composed, ScreenComposer, Variable};
use p4kx_testkit::{
    TestCanvas, TestCondition, TestDim, TestModifier, TestOp, TestRecord, TestStyleEntry,
    TestValue, TestWidget,
};

use crate::support::{
    BLUE, RED, SCREEN, TestResult, blue_at, database, empty_at, guid_text, red_at, stage,
};

const RED_CANVAS: u128 = SCREEN + 0x100;
const BLUE_CANVAS: u128 = SCREEN + 0x200;

const fn square(widget: TestWidget) -> TestWidget {
    widget.size(TestDim::Fixed(10.0), TestDim::Fixed(10.0))
}

fn filled(guid: u128, name: &str, colour: p4kx_testkit::TestColour) -> TestRecord {
    TestCanvas::new(guid, name, 10.0, 10.0)
        .widget(square(TestWidget::display("fill").background(colour)))
        .record()
}

fn holder(with_canvas: Option<u128>) -> TestWidget {
    let widget = square(TestWidget::new("BuildingBlocks_WidgetCanvas", "holder"));
    match with_canvas {
        Some(guid) => widget.with("canvas", TestValue::Reference(guid)),
        None => widget,
    }
}

fn compose(
    outer: &TestCanvas,
    others: Vec<TestRecord>,
    settings: &[&str],
) -> Result<Composed, Box<dyn std::error::Error>> {
    let bytes = database(std::iter::once(outer.record()).chain(others));
    let forge = DataForge::open(&bytes)?;
    let variables = settings
        .iter()
        .map(|setting| Ok(setting.parse::<Variable>()?))
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    Ok(ScreenComposer::new(&forge)
        .with_variables(&variables)
        .compose(&guid_text(SCREEN), None)?)
}

fn screen() -> TestCanvas {
    TestCanvas::new(SCREEN, "outer", 10.0, 10.0)
}

fn swap_to(canvas: u128) -> TestStyleEntry {
    TestStyleEntry::new("swap")
        .when(TestCondition::Type("Canvas"))
        .set(TestModifier::Canvas(canvas))
}

fn both() -> Vec<TestRecord> {
    vec![
        filled(RED_CANVAS, "red", RED),
        filled(BLUE_CANVAS, "blue", BLUE),
    ]
}

#[test]
fn a_style_entry_puts_a_canvas_in_a_canvas_widget_that_holds_none() -> TestResult {
    let outer = screen().embedded(swap_to(RED_CANVAS)).widget(holder(None));
    let composed = compose(&outer, both(), &[])?;
    assert!(red_at(&composed.rasterize(&stage())?.image, 5, 5));
    assert_eq!(composed.report().missing_canvases.first(), None);
    Ok(())
}

#[test]
fn a_style_entry_beats_the_canvas_of_the_widget() -> TestResult {
    let outer = screen()
        .embedded(swap_to(RED_CANVAS))
        .widget(holder(Some(BLUE_CANVAS)));
    let image = compose(&outer, both(), &[])?.rasterize(&stage())?.image;
    assert!(red_at(&image, 5, 5));
    Ok(())
}

fn bound() -> TestCanvas {
    screen()
        .operation("flag", TestOp::NamedBoolVar("holderon"))
        .operation(
            "pick",
            TestOp::CanvasFromBool("flag", RED_CANVAS, BLUE_CANVAS),
        )
        .widget(holder(None))
        .operation("set", TestOp::CanvasField("holder", "pick"))
}

#[test]
fn a_binding_picks_the_canvas_by_a_game_variable() -> TestResult {
    let off = compose(&bound(), both(), &[])?;
    assert!(blue_at(&off.rasterize(&stage())?.image, 5, 5), "at rest");
    let on = compose(&bound(), both(), &["holderon=true"])?;
    assert!(red_at(&on.rasterize(&stage())?.image, 5, 5), "set");
    assert_eq!(on.report().missing_canvases.first(), None);
    Ok(())
}

#[test]
fn a_canvas_that_only_a_binding_can_reach_is_surveyed_for_variables() -> TestResult {
    let behind = TestCanvas::new(BLUE_CANVAS, "blue", 10.0, 10.0)
        .operation("deep", TestOp::NamedBoolVar("deep"));
    let composed = compose(
        &bound(),
        vec![filled(RED_CANVAS, "red", RED), behind.record()],
        &["deep=1"],
    )?;
    assert_eq!(composed.report().unknown_variables().count(), 0);
    Ok(())
}

#[test]
fn a_canvas_that_run_time_data_names_is_left_as_it_is_and_listed() -> TestResult {
    let outer = screen()
        .operation("name", TestOp::TextVar)
        .widget(holder(Some(RED_CANVAS)))
        .operation("set", TestOp::CanvasField("holder", "name"));
    let composed = compose(&outer, both(), &[])?;
    assert!(red_at(&composed.rasterize(&stage())?.image, 5, 5));
    assert!(composed.report().unsupported.iter().any(|entry| {
        entry.element == "Binding" && entry.reason.contains("canvas that only the game names")
    }));
    Ok(())
}

fn namespaced() -> (TestCanvas, Vec<TestRecord>) {
    let outer = TestCanvas::new(SCREEN, "outer", 20.0, 10.0)
        .widget(TestWidget::display("row").flex("Row", "Start", "Start", 0.0))
        .operation("power", TestOp::NamedNumVar("power"))
        .operation("on", TestOp::NumIs("power", "Greater", 0.0))
        .widget(square(TestWidget::display("a").background(RED)).parent("row"))
        .operation("show_a", TestOp::BoolField("a", "Instantiated", "on"))
        .widget(holder(Some(RED_CANVAS)).parent("row"));
    let inner = TestCanvas::new(RED_CANVAS, "inner", 10.0, 10.0)
        .operation("power", TestOp::NamedNumVar("power"))
        .operation("on", TestOp::NumIs("power", "Greater", 0.0))
        .widget(square(TestWidget::display("b").background(BLUE)))
        .operation("show_b", TestOp::BoolField("b", "Instantiated", "on"));
    (outer, vec![inner.record()])
}

#[test]
fn a_namespace_reaches_the_variables_of_one_canvas_only() -> TestResult {
    let (outer, others) = namespaced();
    let both = compose(&outer, others.clone(), &["power=1"])?
        .rasterize(&stage())?
        .image;
    assert!(
        red_at(&both, 5, 5) && blue_at(&both, 15, 5),
        "no namespace: both"
    );
    let only_inner = compose(&outer, others.clone(), &["inner::power=1"])?
        .rasterize(&stage())?
        .image;
    assert!(
        blue_at(&only_inner, 5, 5) && empty_at(&only_inner, 15, 5),
        "the hidden widget leaves the row, so the canvas moves up"
    );
    let only_outer = compose(&outer, others, &["outer::power=1"])?
        .rasterize(&stage())?
        .image;
    assert!(red_at(&only_outer, 5, 5) && empty_at(&only_outer, 15, 5));
    Ok(())
}

#[test]
fn a_namespace_that_no_canvas_has_is_reported_as_unknown() -> TestResult {
    let (outer, others) = namespaced();
    let composed = compose(&outer, others, &["inner::power=1", "Nowhere::power=1"])?;
    assert_eq!(
        composed.report().unknown_variables().collect::<Vec<_>>(),
        ["Nowhere::power"]
    );
    Ok(())
}
