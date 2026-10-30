// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Game variables a caller sets before the bindings are evaluated.

use p4kx_dataforge::DataForge;
use p4kx_screens::{
    Composed, MAX_VARIABLES, ScreenComposer, ScreenError, Variable, VariableInForce,
};
use p4kx_testkit::{TestCanvas, TestDim, TestOp, TestValue, TestWidget};

use crate::support::{RED, SCREEN, TestResult, database, empty_at, guid_text, red_at, stage};

const INNER: u128 = SCREEN + 0x100;

fn canvas() -> TestCanvas {
    TestCanvas::new(SCREEN, "variables", 10.0, 10.0)
}

const fn square(widget: TestWidget) -> TestWidget {
    widget.size(TestDim::Fixed(10.0), TestDim::Fixed(10.0))
}

fn variables(settings: &[&str]) -> Result<Vec<Variable>, Box<dyn std::error::Error>> {
    settings
        .iter()
        .map(|setting| Ok(setting.parse::<Variable>()?))
        .collect()
}

fn compose_with(
    records: Vec<p4kx_testkit::TestRecord>,
    settings: &[&str],
) -> Result<Composed, Box<dyn std::error::Error>> {
    let bytes = database(records);
    let forge = DataForge::open(&bytes)?;
    Ok(ScreenComposer::new(&forge)
        .with_variables(&variables(settings)?)
        .compose(&guid_text(SCREEN), None)?)
}

fn powered_screen() -> TestCanvas {
    canvas()
        .operation("power", TestOp::NamedNumVar("PowerState"))
        .operation("on", TestOp::NumIs("power", "Greater", 0.0))
        .widget(square(TestWidget::display("mfd").background(RED)))
        .operation("show", TestOp::BoolField("mfd", "Instantiated", "on"))
}

fn composed(settings: &[&str]) -> Result<Composed, Box<dyn std::error::Error>> {
    compose_with(vec![powered_screen().record()], settings)
}

#[test]
fn a_variable_that_is_set_turns_the_screen_on_and_the_report_lists_it() -> TestResult {
    let off = composed(&[])?;
    assert!(empty_at(&off.rasterize(&stage())?.image, 5, 5));
    assert_eq!(off.report().variables.first(), None);
    let on = composed(&["powerstate=1"])?;
    assert!(red_at(&on.rasterize(&stage())?.image, 5, 5));
    assert_eq!(
        on.report().variables,
        [VariableInForce {
            name: "powerstate".to_owned(),
            value: "1".to_owned(),
            used: true,
        }]
    );
    assert_eq!(on.report().unknown_variables().count(), 0);
    assert!(
        on.report()
            .unsupported
            .iter()
            .all(|entry| !entry.reason.starts_with("not drawn at rest")),
        "a screen that is drawn is not listed as gone"
    );
    Ok(())
}

#[test]
fn a_name_is_the_last_segment_of_the_path_a_binding_reads() -> TestResult {
    let screen = canvas()
        .operation("power", TestOp::NamedNumVar("/seatdashboard/powerstate"))
        .operation("on", TestOp::NumIs("power", "Greater", 0.0))
        .widget(square(TestWidget::display("mfd").background(RED)))
        .operation("show", TestOp::BoolField("mfd", "Instantiated", "on"));
    for (setting, drawn) in [
        ("powerstate=1", true),
        ("/seatdashboard/powerstate=1", true),
        ("/other/powerstate=1", false),
    ] {
        let composed = compose_with(vec![screen.record()], &[setting])?;
        let image = composed.rasterize(&stage())?.image;
        assert_eq!(red_at(&image, 5, 5), drawn, "{setting}");
        assert_eq!(composed.report().unknown_variables().count() == 0, drawn);
    }
    Ok(())
}

#[test]
fn a_variable_that_no_canvas_reads_is_reported_and_changes_nothing() -> TestResult {
    let composed = composed(&["powerstat=1", "powerstate=0"])?;
    assert!(empty_at(&composed.rasterize(&stage())?.image, 5, 5));
    assert_eq!(
        composed.report().unknown_variables().collect::<Vec<_>>(),
        ["powerstat"]
    );
    Ok(())
}

#[test]
fn the_last_setting_of_a_name_wins_whatever_its_case() -> TestResult {
    let composed = composed(&["powerstate=0", "POWERSTATE=1"])?;
    assert!(red_at(&composed.rasterize(&stage())?.image, 5, 5));
    assert_eq!(composed.report().variables.len(), 1);
    assert_eq!(composed.report().variables[0].value, "1");
    Ok(())
}

#[test]
fn a_value_of_the_wrong_kind_is_listed_and_the_widget_keeps_its_own() -> TestResult {
    let composed = composed(&["powerstate=on"])?;
    assert!(
        red_at(&composed.rasterize(&stage())?.image, 5, 5),
        "the binding is unknown, so the widget stays"
    );
    assert!(
        composed
            .report()
            .unsupported
            .iter()
            .any(|entry| entry.element == "Binding" && entry.reason.contains("not a number")),
        "{:?}",
        composed.report().unsupported
    );
    Ok(())
}

#[test]
fn a_variable_of_a_canvas_that_is_not_entered_is_still_known() -> TestResult {
    let inner = TestCanvas::new(INNER, "inner", 10.0, 10.0)
        .operation("deep", TestOp::NamedNumVar("deep"))
        .operation("deeper", TestOp::NumIs("deep", "Greater", 0.0))
        .widget(square(TestWidget::display("content").background(RED)))
        .operation("bind", TestOp::BoolField("content", "IsActive", "deeper"));
    let outer = canvas()
        .operation("flag", TestOp::BoolVar)
        .widget(
            square(TestWidget::new("BuildingBlocks_WidgetCanvas", "holder"))
                .with("canvas", TestValue::Reference(INNER)),
        )
        .operation("hide", TestOp::BoolField("holder", "Instantiated", "flag"));
    let composed = compose_with(vec![outer.record(), inner.record()], &["deep=1", "other=1"])?;
    assert_eq!(
        composed.report().unknown_variables().collect::<Vec<_>>(),
        ["other"]
    );
    Ok(())
}

#[test]
fn more_variables_than_the_limit_are_refused() -> TestResult {
    let settings: Vec<String> = (0..=MAX_VARIABLES).map(|at| format!("v{at}=1")).collect();
    let settings: Vec<&str> = settings.iter().map(String::as_str).collect();
    match composed(&settings) {
        Err(error) => match error.downcast_ref::<ScreenError>() {
            Some(ScreenError::TooManyVariables { count, max }) => {
                assert_eq!((*count, *max), (MAX_VARIABLES + 1, MAX_VARIABLES));
                Ok(())
            }
            other => Err(format!("another error: {other:?}").into()),
        },
        Ok(_) => Err("the settings were taken".into()),
    }
}
