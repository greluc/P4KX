// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The state a canvas has at rest: what its `operations` bind to `Instantiated`, `IsActive`,
//! `Alpha` and the parameters of nested canvases, with every game variable at its zero value.

use p4kx_dataforge::DataForge;
use p4kx_screens::{Composed, ScreenComposer};
use p4kx_testkit::{
    TestCanvas, TestCondition, TestDim, TestModifier, TestOp, TestParam, TestStyleEntry, TestValue,
    TestWidget,
};

use crate::support::{
    GREEN, RED, SCREEN, TestResult, compose, database, empty_at, green_at, guid_text, near, pixel,
    red_at, render, stage,
};

const INNER: u128 = SCREEN + 0x100;

fn canvas() -> TestCanvas {
    TestCanvas::new(SCREEN, "rest", 40.0, 10.0)
}

const fn square(widget: TestWidget) -> TestWidget {
    widget.size(TestDim::Fixed(10.0), TestDim::Fixed(10.0))
}

fn row() -> TestWidget {
    TestWidget::display("row").flex("Row", "Start", "Start", 0.0)
}

#[test]
fn a_bound_instantiated_field_takes_the_value_the_variables_have_at_rest() -> TestResult {
    let image = render(
        &canvas()
            .operation("flag", TestOp::BoolVar)
            .operation("opposite", TestOp::Invert("flag"))
            .widget(row())
            .widget(square(
                TestWidget::display("gone").parent("row").background(RED),
            ))
            .widget(square(
                TestWidget::display("there").parent("row").background(GREEN),
            ))
            .operation("hide", TestOp::BoolField("gone", "Instantiated", "flag"))
            .operation(
                "show",
                TestOp::BoolField("there", "Instantiated", "opposite"),
            ),
    )?;
    assert!(
        green_at(&image, 5, 5),
        "the widget that is not instantiated takes no room"
    );
    assert!(empty_at(&image, 15, 5));
    Ok(())
}

#[test]
fn a_bound_is_active_field_hides_the_widget() -> TestResult {
    let image = render(
        &canvas()
            .operation("flag", TestOp::BoolVar)
            .widget(row())
            .widget(square(
                TestWidget::display("off").parent("row").background(RED),
            ))
            .widget(square(
                TestWidget::display("on").parent("row").background(GREEN),
            ))
            .operation("bind", TestOp::BoolField("off", "IsActive", "flag")),
    )?;
    assert!(green_at(&image, 5, 5));
    assert!(empty_at(&image, 15, 5));
    Ok(())
}

#[test]
fn a_bound_alpha_takes_the_number_the_operations_make_of_the_variables() -> TestResult {
    let image = render(
        &canvas()
            .operation("flag", TestOp::BoolVar)
            .operation("fade", TestOp::NumFromBool("flag", 1.0, 0.5))
            .widget(square(TestWidget::display("w").background(RED)))
            .operation("bind", TestOp::NumField("w", "Alpha", "fade")),
    )?;
    assert!(near(pixel(&image, 5, 5), [255, 0, 0, 128], 1));
    Ok(())
}

#[test]
fn comparisons_see_variables_at_zero_and_parameters_at_their_defaults() -> TestResult {
    let image = render(
        &canvas()
            .operation("count", TestOp::IntVar)
            .operation("none", TestOp::IntIs("count", "Equal", 0))
            .operation("level", TestOp::NumVar)
            .operation("some", TestOp::NumIs("level", "Greater", 0.0))
            .widget(row())
            .widget(square(
                TestWidget::display("a").parent("row").background(RED),
            ))
            .widget(square(
                TestWidget::display("b").parent("row").background(GREEN),
            ))
            .operation("keep", TestOp::BoolField("a", "IsActive", "none"))
            .operation("drop", TestOp::BoolField("b", "IsActive", "some")),
    )?;
    assert!(red_at(&image, 5, 5), "the integer is zero, so a stays");
    assert!(
        empty_at(&image, 15, 5),
        "the number is not above zero, so b goes"
    );
    Ok(())
}

#[test]
fn a_style_entry_beats_a_bound_value_which_beats_the_field() -> TestResult {
    let show = TestStyleEntry::new("show")
        .when(TestCondition::Type("Base"))
        .set(TestModifier::Active(true));
    let image = render(
        &canvas()
            .embedded(show)
            .operation("flag", TestOp::BoolVar)
            .widget(row())
            .widget(square(TestWidget::display("a").parent("row").background(RED)).hidden())
            .operation("bind", TestOp::BoolField("a", "IsActive", "flag")),
    )?;
    assert!(
        red_at(&image, 5, 5),
        "the entry shows what the binding hides"
    );
    Ok(())
}

fn inner() -> TestCanvas {
    TestCanvas::new(INNER, "inner", 10.0, 10.0)
        .operation("shown", TestOp::BoolParam("ParamInput0", true))
        .widget(square(TestWidget::display("content").background(RED)))
        .operation("bind", TestOp::BoolField("content", "IsActive", "shown"))
}

fn holder(name: &str) -> TestWidget {
    square(TestWidget::new("BuildingBlocks_WidgetCanvas", name))
        .parent("row")
        .with("canvas", TestValue::Reference(INNER))
}

fn compose_both(outer: &TestCanvas) -> Result<Composed, Box<dyn std::error::Error>> {
    let bytes = database([outer.record(), inner().record()]);
    let forge = DataForge::open(&bytes)?;
    Ok(ScreenComposer::new(&forge).compose(&guid_text(SCREEN), None)?)
}

#[test]
fn a_nested_canvas_reads_the_parameters_its_widget_hands_it_else_the_defaults() -> TestResult {
    let outer = canvas()
        .widget(row())
        .widget(holder("given").params(&[TestParam::Bool("ParamInput0", false)]))
        .widget(holder("default"))
        .widget(holder("same").params(&[TestParam::Bool("ParamInput0", true)]));
    let image = compose_both(&outer)?.rasterize(&stage())?.image;
    assert!(
        empty_at(&image, 5, 5),
        "the parameter that is false hides the content"
    );
    assert!(red_at(&image, 15, 5), "the default shows it");
    assert!(red_at(&image, 25, 5), "the given true shows it");
    assert!(empty_at(&image, 35, 5));
    Ok(())
}

#[test]
fn a_parameter_that_the_canvas_binds_to_a_variable_reaches_the_nested_canvas() -> TestResult {
    let outer = canvas()
        .operation("flag", TestOp::BoolVar)
        .widget(row())
        .widget(holder("bound"))
        .widget(holder("plain"))
        .operation("param", TestOp::BoolField("bound", "ParamInput0", "flag"));
    let image = compose_both(&outer)?.rasterize(&stage())?.image;
    assert!(empty_at(&image, 5, 5), "the bound canvas is hidden at rest");
    assert!(red_at(&image, 15, 5), "the other keeps its default");
    Ok(())
}

#[test]
fn what_is_not_evaluated_is_reported_and_leaves_the_widget_as_it_is() -> TestResult {
    let composed = compose(
        &canvas()
            .operation("mystery", TestOp::Other)
            .operation("flag", TestOp::BoolVar)
            .operation("size", TestOp::NumFromBool("flag", 1.0, 2.0))
            .widget(square(TestWidget::display("w").background(RED)))
            .operation("bind", TestOp::BoolField("w", "IsActive", "mystery"))
            .operation("grow", TestOp::NumField("w", "SizeX", "size")),
    )?;
    let reasons: Vec<&str> = composed
        .report()
        .unsupported
        .iter()
        .filter(|entry| entry.element == "Binding")
        .map(|entry| entry.reason.as_str())
        .collect();
    assert!(
        reasons.iter().any(|reason| reason.contains("AudioEvent")),
        "{reasons:?}"
    );
    assert!(
        reasons
            .iter()
            .any(|reason| reason.contains("layout fields")),
        "{reasons:?}"
    );
    assert_eq!(composed.report().shapes, 1, "the widget is still drawn");
    Ok(())
}

#[test]
fn a_root_that_a_binding_takes_out_of_the_screen_is_listed_and_evaluated_bindings_are_not()
-> TestResult {
    let composed = compose(
        &canvas()
            .operation("flag", TestOp::BoolVar)
            .widget(square(TestWidget::display("gone").background(RED)))
            .widget(square(TestWidget::display("dim").background(GREEN)))
            .operation("hide", TestOp::BoolField("gone", "Instantiated", "flag"))
            .operation("fade", TestOp::NumFromBool("flag", 1.0, 0.4))
            .operation("alpha", TestOp::NumField("dim", "Alpha", "fade")),
    )?;
    let listed: Vec<(&str, &str)> = composed
        .report()
        .unsupported
        .iter()
        .map(|entry| (entry.element.as_str(), entry.reason.as_str()))
        .collect();
    assert!(
        listed.iter().any(|(element, reason)| *element == "Binding"
            && reason.starts_with("not drawn at rest: Instantiated")),
        "{listed:?}"
    );
    assert_eq!(listed.len(), 1, "the evaluated alpha is no gap: {listed:?}");
    Ok(())
}

#[test]
fn and_is_false_and_or_is_true_when_one_input_decides_it() -> TestResult {
    let composed = compose(
        &canvas()
            .operation("flag", TestOp::BoolVar)
            .operation("mystery", TestOp::Other)
            .operation("both", TestOp::And(vec!["flag", "mystery"]))
            .operation("either", TestOp::Or(vec!["flag", "mystery"]))
            .widget(row())
            .widget(square(
                TestWidget::display("a").parent("row").background(RED),
            ))
            .widget(square(
                TestWidget::display("b").parent("row").background(GREEN),
            ))
            .operation("bind_a", TestOp::BoolField("a", "IsActive", "both"))
            .operation("bind_b", TestOp::BoolField("b", "IsActive", "either")),
    )?;
    let image = composed.rasterize(&stage())?.image;
    assert!(
        green_at(&image, 5, 5),
        "the false operand decides the and, so a is hidden, and the unknown or keeps b"
    );
    Ok(())
}
