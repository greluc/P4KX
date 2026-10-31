// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! State tags a canvas binds to its widgets, and the animations a style entry settles.

use p4kx_dataforge::DataForge;
use p4kx_screens::{Composed, ScreenComposer, Variable};
use p4kx_testkit::{
    TestCanvas, TestCondition, TestDim, TestModifier, TestOp, TestStyleEntry, TestWidget,
    tag_record,
};

use crate::support::{
    BLUE, GREEN, RED, SCREEN, TestResult, database, empty_at, green_at, guid_text, red_at, stage,
};

const ON: u128 = SCREEN + 0x200;
const OFF: u128 = SCREEN + 0x201;

fn canvas() -> TestCanvas {
    TestCanvas::new(SCREEN, "state_tags", 10.0, 10.0)
}

fn swatch() -> TestWidget {
    TestWidget::display("swatch").size(TestDim::Fixed(10.0), TestDim::Fixed(10.0))
}

fn paint(name: &str, tag: u128, colour: p4kx_testkit::TestColour) -> TestStyleEntry {
    TestStyleEntry::new(name)
        .when(TestCondition::Tag(tag))
        .set(TestModifier::EnableBackground(true))
        .set(TestModifier::Background(colour))
}

fn compose(canvas: &TestCanvas, settings: &[&str]) -> Result<Composed, Box<dyn std::error::Error>> {
    let records = vec![
        canvas.record(),
        tag_record(ON, "On"),
        tag_record(OFF, "Off"),
    ];
    let bytes = database(records);
    let forge = DataForge::open(&bytes)?;
    let variables = settings
        .iter()
        .map(|setting| Ok(setting.parse::<Variable>()?))
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    Ok(ScreenComposer::new(&forge)
        .with_variables(&variables)
        .compose(&guid_text(SCREEN), None)?)
}

fn bound() -> TestCanvas {
    canvas()
        .embedded(paint("on", ON, RED))
        .embedded(paint("off", OFF, GREEN))
        .operation("flag", TestOp::BoolVar)
        .operation("pick", TestOp::TagFromBool("flag", ON, OFF))
        .widget(swatch())
        .operation(
            "bind",
            TestOp::TagField("swatch", "PrimaryStateTag", "pick"),
        )
}

#[test]
fn a_state_tag_selects_the_style_entry_that_colours_the_widget() -> TestResult {
    let at_rest = compose(&bound(), &[])?;
    assert!(green_at(&at_rest.rasterize(&stage())?.image, 5, 5));
    let on = compose(&bound(), &["variable=true"])?;
    assert!(red_at(&on.rasterize(&stage())?.image, 5, 5));
    Ok(())
}

#[test]
fn a_state_tag_is_not_listed_as_left_out() -> TestResult {
    let composed = compose(&bound(), &[])?;
    let listed = composed
        .report()
        .unsupported
        .iter()
        .any(|entry| entry.element == "Binding" && entry.reason.contains("state tag"));
    assert!(!listed, "{:?}", composed.report().unsupported);
    Ok(())
}

fn animated(fill: &'static str, direction: &'static str, from: f32, to: f32) -> TestCanvas {
    canvas()
        .embedded(
            paint("shown", ON, BLUE).set(TestModifier::AlphaAnimation(fill, direction, from, to)),
        )
        .operation("flag", TestOp::BoolVar)
        .operation("pick", TestOp::TagFromBool("flag", ON, OFF))
        .widget(swatch())
        .operation(
            "bind",
            TestOp::TagField("swatch", "PrimaryStateTag", "pick"),
        )
}

#[test]
fn an_animation_that_holds_its_last_frame_settles_on_it() -> TestResult {
    let settled = compose(
        &animated("Forward", "Forward", 0.0, 1.0),
        &["variable=true"],
    )?;
    assert!(
        crate::support::blue_at(&settled.rasterize(&stage())?.image, 5, 5),
        "from nothing to opaque: opaque"
    );
    let faded = compose(
        &animated("Forward", "Forward", 1.0, 0.0),
        &["variable=true"],
    )?;
    assert!(
        empty_at(&faded.rasterize(&stage())?.image, 5, 5),
        "from opaque to nothing: nothing"
    );
    let reversed = compose(&animated("Both", "Reverse", 1.0, 0.0), &["variable=true"])?;
    assert!(
        crate::support::blue_at(&reversed.rasterize(&stage())?.image, 5, 5),
        "a reverse animation settles on its first frame"
    );
    Ok(())
}

#[test]
fn an_animation_that_does_not_hold_its_last_frame_leaves_nothing() -> TestResult {
    let released = compose(&animated("None", "Forward", 1.0, 0.0), &["variable=true"])?;
    assert!(crate::support::blue_at(
        &released.rasterize(&stage())?.image,
        5,
        5
    ));
    Ok(())
}
