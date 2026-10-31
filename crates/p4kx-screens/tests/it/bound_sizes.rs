// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Sizes, anchors and the other number fields of the layout that a canvas binds to its widgets.

use p4kx_dataforge::DataForge;
use p4kx_screens::{Composed, ScreenComposer, Variable};
use p4kx_testkit::{TestCanvas, TestDim, TestModifier, TestOp, TestStyleEntry, TestWidget};

use crate::support::{RED, SCREEN, TestResult, database, empty_at, guid_text, red_at, stage};

fn canvas() -> TestCanvas {
    TestCanvas::new(SCREEN, "bound_sizes", 40.0, 10.0)
}

fn bar() -> TestWidget {
    TestWidget::display("bar")
        .size(TestDim::Fixed(10.0), TestDim::Fixed(10.0))
        .background(RED)
}

fn sized(canvas: TestCanvas, field: &'static str) -> TestCanvas {
    canvas
        .operation("flag", TestOp::BoolVar)
        .operation("size", TestOp::NumFromBool("flag", 30.0, 20.0))
        .widget(bar())
        .operation("bind", TestOp::NumField("bar", field, "size"))
}

fn compose(canvas: &TestCanvas, settings: &[&str]) -> Result<Composed, Box<dyn std::error::Error>> {
    let bytes = database([canvas.record()]);
    let forge = DataForge::open(&bytes)?;
    let variables = settings
        .iter()
        .map(|setting| Ok(setting.parse::<Variable>()?))
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    Ok(ScreenComposer::new(&forge)
        .with_variables(&variables)
        .compose(&guid_text(SCREEN), None)?)
}

#[test]
fn a_bound_size_sets_the_width_the_variables_decide() -> TestResult {
    let at_rest = compose(&sized(canvas(), "SizeX"), &[])?
        .rasterize(&stage())?
        .image;
    assert!(red_at(&at_rest, 15, 5), "the width is 20 at rest");
    assert!(empty_at(&at_rest, 25, 5));
    let set = compose(&sized(canvas(), "SizeX"), &["variable=true"])?
        .rasterize(&stage())?
        .image;
    assert!(red_at(&set, 25, 5), "the width is 30 with the variable");
    assert!(empty_at(&set, 35, 5));
    Ok(())
}

#[test]
fn a_style_entry_beats_a_bound_size() -> TestResult {
    let entry = TestStyleEntry::new("narrow").set(TestModifier::Number("SizeX", 5.0));
    let image = compose(&sized(canvas().embedded(entry), "SizeX"), &[])?
        .rasterize(&stage())?
        .image;
    assert!(red_at(&image, 2, 5));
    assert!(
        empty_at(&image, 8, 5),
        "the entry's 5 wins over the bound 20"
    );
    Ok(())
}

#[test]
fn a_size_that_cannot_be_decided_is_listed_and_the_widget_keeps_its_size() -> TestResult {
    let composed = compose(
        &canvas()
            .operation("mystery", TestOp::Other)
            .widget(bar())
            .operation("bind", TestOp::NumField("bar", "SizeX", "mystery")),
        &[],
    )?;
    let image = composed.rasterize(&stage())?.image;
    assert!(red_at(&image, 5, 5));
    assert!(empty_at(&image, 15, 5), "the width stays 10");
    assert!(
        composed
            .report()
            .unsupported
            .iter()
            .any(|entry| entry.element == "Binding"
                && entry
                    .reason
                    .contains("the bound field SizeX is not evaluated")),
        "{:?}",
        composed.report().unsupported
    );
    Ok(())
}

#[test]
fn a_bound_depth_stays_listed_as_not_evaluated() -> TestResult {
    let composed = compose(&sized(canvas(), "PosZ"), &[])?;
    assert!(
        composed
            .report()
            .unsupported
            .iter()
            .any(|entry| entry.element == "Binding" && entry.reason.contains("layout fields")),
        "{:?}",
        composed.report().unsupported
    );
    Ok(())
}
