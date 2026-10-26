// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde_json::Value;

use super::assembly_metrics as m;

const DESKTOP: &str = include_str!("../../../../docs/design/p4kx-desktop-spec.json");

fn assembly_view() -> Value {
    let spec: Value =
        serde_json::from_str(DESKTOP).unwrap_or_else(|error| panic!("the desktop spec: {error}"));
    spec["assemblyView"].clone()
}

#[test]
fn the_assembly_view_metrics_match_the_desktop_spec() {
    let spec = assembly_view();

    for (key, value) in [
        ("viewHeight", m::ASSEMBLY_VIEW_HEIGHT),
        ("treeMinRows", m::ASSEMBLY_TREE_MIN_ROWS),
        ("barRows", m::ASSEMBLY_BAR_ROWS),
        ("panelHeight", m::ASSEMBLY_PANEL_HEIGHT),
        ("panelMinHeight", m::ASSEMBLY_PANEL_MIN_HEIGHT),
        ("treeMinHeight", m::ASSEMBLY_TREE_MIN_HEIGHT),
        ("minPartPixels", crate::MIN_PART_PIXELS),
        ("panelShare", m::ASSEMBLY_PANEL_SHARE),
    ] {
        let expected = spec[key].as_f64();
        assert!(
            expected.is_some_and(|want| (want - f64::from(value)).abs() < 0.01),
            "assemblyView.{key} is {expected:?}, the theme has {value}"
        );
    }
}

#[test]
fn the_replace_and_progress_metrics_match_the_desktop_spec() {
    let spec: Value =
        serde_json::from_str(DESKTOP).unwrap_or_else(|error| panic!("the desktop spec: {error}"));

    for (element, key, value) in [
        ("entityReplace", "listHeight", m::REPLACE_LIST_HEIGHT),
        ("entityReplace", "headGap", m::REPLACE_HEAD_GAP),
    ] {
        let expected = spec[element][key].as_f64();
        assert!(
            expected.is_some_and(|want| (want - f64::from(value)).abs() < 0.01),
            "{element}.{key} is {expected:?}, the theme has {value}"
        );
    }
    assert_eq!(
        spec["progressBar"]["sweepPeriodFrom"].as_str(),
        Some("spinner.periodMs"),
        "a sweep takes one turn of the spinner"
    );
    assert_eq!(
        spec["entityReplace"]["decision"].as_str(),
        Some("D.57"),
        "decision"
    );
}

#[test]
fn the_assembly_view_names_its_decision_and_parts() {
    let spec = assembly_view();

    assert_eq!(spec["decision"].as_str(), Some("D.47"), "decision");
    assert_eq!(
        spec["interiorDecision"].as_str(),
        Some("D.48"),
        "the decision of the Interior"
    );
    for key in [
        "dsClass",
        "layout",
        "bar",
        "treeActions",
        "culling",
        "messages",
        "interior",
        "interiorExport",
    ] {
        assert!(
            spec[key].as_str().is_some_and(|text| !text.is_empty()),
            "assemblyView.{key} is described"
        );
    }
}
