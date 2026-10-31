// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Canvases that swap into each other or into themselves, and the widget cap on shared canvases.

use p4kx_dataforge::DataForge;
use p4kx_screens::{Composed, Limits, ScreenComposer};
use p4kx_testkit::{
    TestCanvas, TestCondition, TestDim, TestModifier, TestRecord, TestStyleEntry, TestValue,
    TestWidget,
};

use crate::support::{RED, SCREEN, TestResult, database, guid_text, red_at, stage};

const A: u128 = SCREEN + 0x100;
const B: u128 = SCREEN + 0x200;
const C: u128 = SCREEN + 0x300;
const D: u128 = SCREEN + 0x400;

const fn square(widget: TestWidget) -> TestWidget {
    widget.size(TestDim::Fixed(10.0), TestDim::Fixed(10.0))
}

fn holder(canvas: Option<u128>) -> TestWidget {
    let widget = square(TestWidget::new("BuildingBlocks_WidgetCanvas", "holder"));
    match canvas {
        Some(guid) => widget.with("canvas", TestValue::Reference(guid)),
        None => widget,
    }
}

fn red_base() -> TestWidget {
    square(TestWidget::display("base").background(RED))
}

fn canvas(guid: u128, name: &str) -> TestCanvas {
    TestCanvas::new(guid, name, 10.0, 10.0)
}

fn compose(
    outer: &TestCanvas,
    others: Vec<TestRecord>,
    limits: Limits,
) -> Result<Composed, Box<dyn std::error::Error>> {
    let bytes = database(std::iter::once(outer.record()).chain(others));
    let forge = DataForge::open(&bytes)?;
    Ok(ScreenComposer::new(&forge)
        .with_limits(limits)
        .compose(&guid_text(SCREEN), None)?)
}

fn contains_itself(composed: &Composed) -> bool {
    composed
        .report()
        .missing_canvases
        .iter()
        .any(|name| name.contains("a canvas that contains itself"))
}

#[test]
fn two_canvases_that_hold_each_other_end_and_are_named() -> TestResult {
    let outer = canvas(SCREEN, "outer")
        .widget(red_base())
        .widget(holder(Some(A)));
    let others = vec![
        canvas(A, "a").widget(holder(Some(B))).record(),
        canvas(B, "b").widget(holder(Some(A))).record(),
    ];
    let composed = compose(&outer, others, Limits::DEFAULT)?;
    assert!(contains_itself(&composed), "{:?}", composed.report());
    assert!(!composed.report().truncated);
    assert!(red_at(&composed.rasterize(&stage())?.image, 5, 5));
    Ok(())
}

#[test]
fn a_style_entry_that_swaps_a_canvas_into_itself_ends_and_is_named() -> TestResult {
    let entry = TestStyleEntry::new("swap")
        .when(TestCondition::Type("Canvas"))
        .set(TestModifier::Canvas(SCREEN));
    let outer = canvas(SCREEN, "outer")
        .embedded(entry)
        .widget(red_base())
        .widget(holder(None));
    let composed = compose(&outer, Vec::new(), Limits::DEFAULT)?;
    assert!(contains_itself(&composed), "{:?}", composed.report());
    assert!(red_at(&composed.rasterize(&stage())?.image, 5, 5));
    Ok(())
}

fn diamond() -> (TestCanvas, Vec<TestRecord>) {
    let outer = canvas(SCREEN, "outer")
        .widget(holder(Some(B)))
        .widget(holder(Some(C)));
    let others = vec![
        canvas(B, "b").widget(holder(Some(D))).record(),
        canvas(C, "c").widget(holder(Some(D))).record(),
        canvas(D, "d").widget(red_base()).record(),
    ];
    (outer, others)
}

#[test]
fn a_canvas_reached_twice_counts_its_widgets_twice_against_the_cap() -> TestResult {
    let (outer, others) = diamond();
    let whole = compose(&outer, others.clone(), Limits::DEFAULT)?;
    assert_eq!(whole.report().widgets, 6, "2 holders, 2 holders, 2 bases");
    assert_eq!(whole.report().canvases, 5, "D is entered through B and C");
    assert!(!whole.report().truncated);

    let capped = Limits {
        widgets: whole.report().widgets - 1,
        ..Limits::DEFAULT
    };
    let cut = compose(&outer, others, capped)?;
    assert!(cut.report().truncated);
    assert!(cut.report().widgets < whole.report().widgets);
    Ok(())
}
