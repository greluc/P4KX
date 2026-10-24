// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The work a composition may do is bounded: cascade steps, widgets, depth.

use p4kx_dataforge::DataForge;
use p4kx_screens::{Limits, ScreenComposer};
use p4kx_testkit::{
    TestCanvas, TestColour, TestCondition, TestDim, TestModifier, TestStyleEntry, TestValue,
    TestWidget, tag_record,
};

use crate::support::{TestResult, database, guid_text, near, pixel, stage};

const CANVAS: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0100;
const TAG: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0101;
const RED: TestColour = TestColour::Solid([255, 0, 0, 255]);

fn chain(length: usize, entry: Option<TestStyleEntry>) -> TestCanvas {
    let canvas = TestCanvas::new(CANVAS, "limits", 10.0, 10.0);
    let canvas = entry.into_iter().fold(canvas, TestCanvas::embedded);
    (0..length).fold(canvas, |canvas, at| {
        let widget =
            TestWidget::display(format!("w{at}")).size(TestDim::Fixed(1.0), TestDim::Fixed(1.0));
        let widget = match at.checked_sub(1) {
            Some(before) => widget.parent(format!("w{before}")),
            None => widget.tag(TAG),
        };
        canvas.widget(widget)
    })
}

fn nested_ancestors(levels: usize) -> TestStyleEntry {
    let condition = (0..levels).fold(TestCondition::Tag(TAG), |inner, _| {
        TestCondition::Ancestor(vec![inner])
    });
    TestStyleEntry::new("deep")
        .when(TestCondition::Type("Base"))
        .when(condition)
        .set(TestModifier::EnableBackground(true))
        .set(TestModifier::Background(RED))
}

fn compose(
    canvas: &TestCanvas,
    limits: Limits,
) -> Result<(p4kx_screens::Composed, Vec<u8>), Box<dyn std::error::Error>> {
    let bytes = database([canvas.record(), tag_record(TAG, "Tag")]);
    let composed = {
        let forge = DataForge::open(&bytes)?;
        ScreenComposer::new(&forge)
            .with_limits(limits)
            .compose(&guid_text(CANVAS), None)?
    };
    Ok((composed, bytes))
}

#[test]
fn ancestors_nested_four_deep_apply_over_a_long_chain() -> TestResult {
    let (composed, _bytes) = compose(&chain(60, Some(nested_ancestors(4))), Limits::DEFAULT)?;
    assert!(!composed.report().truncated, "well within the budget");
    let image = composed.rasterize(&stage())?.image;
    assert!(
        near(pixel(&image, 0, 0), [255, 0, 0, 255], 0),
        "the deep widgets are red"
    );
    Ok(())
}

#[test]
fn ancestors_nested_deeper_than_four_are_not_evaluated_and_reported() -> TestResult {
    let (composed, _bytes) = compose(&chain(60, Some(nested_ancestors(6))), Limits::DEFAULT)?;
    let note = composed
        .report()
        .unsupported
        .iter()
        .find(|entry| entry.element == "StyleEntry")
        .ok_or("the entry is not reported")?;
    assert!(note.reason.contains("nested too deep"), "{}", note.reason);
    assert_eq!(composed.report().shapes, 0, "the entry does not apply");
    Ok(())
}

#[test]
fn an_exhausted_cascade_budget_truncates_the_render() -> TestResult {
    let limits = Limits {
        evaluations: 40,
        ..Limits::DEFAULT
    };
    let (composed, _bytes) = compose(&chain(60, Some(nested_ancestors(4))), limits)?;
    assert!(composed.report().truncated, "the budget ran out");
    assert!(
        !composed.report().is_complete(),
        "so the render is incomplete"
    );
    Ok(())
}

#[test]
fn hidden_widgets_count_against_the_widget_limit() -> TestResult {
    let canvas = (0..40).fold(
        TestCanvas::new(CANVAS, "limits", 10.0, 10.0).widget(TestWidget::display("root")),
        |canvas, at| {
            canvas.widget(
                TestWidget::display(format!("h{at}"))
                    .parent("root")
                    .hidden(),
            )
        },
    );
    let limits = Limits {
        widgets: 30,
        ..Limits::DEFAULT
    };
    let (composed, _bytes) = compose(&canvas, limits)?;
    assert!(
        composed.report().truncated,
        "forty hidden children pass the limit of thirty"
    );
    assert!(
        !composed.report().is_complete(),
        "and the render is incomplete"
    );
    Ok(())
}

#[test]
fn a_chain_of_widgets_deeper_than_the_limit_stops_without_overflowing_the_stack() -> TestResult {
    let (composed, _bytes) = compose(&chain(300, None), Limits::DEFAULT)?;
    assert!(composed.report().truncated, "depth 64 is the end");
    assert!(
        composed.report().widgets <= 70,
        "{}",
        composed.report().widgets
    );
    Ok(())
}

#[test]
fn a_chain_of_canvases_deeper_than_the_limit_stops_too() -> TestResult {
    let guid = |at: u128| CANVAS + at;
    let canvases: Vec<_> = (0..300_u128)
        .map(|at| {
            TestCanvas::new(guid(at), format!("link{at}"), 10.0, 10.0)
                .widget(
                    TestWidget::new("BuildingBlocks_WidgetCanvas", "next")
                        .with("canvas", TestValue::Reference(guid(at + 1))),
                )
                .record()
        })
        .collect();
    let bytes = database(canvases);
    let forge = DataForge::open(&bytes)?;
    let composed = ScreenComposer::new(&forge).compose(&guid_text(CANVAS), None)?;
    assert!(composed.report().truncated, "depth 64 is the end");
    assert!(
        composed.report().canvases <= 70,
        "{}",
        composed.report().canvases
    );
    Ok(())
}
