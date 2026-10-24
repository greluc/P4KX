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
fn the_assembly_view_names_its_decision_and_parts() {
    let spec = assembly_view();

    assert_eq!(spec["decision"].as_str(), Some("D.47"), "decision");
    for key in [
        "dsClass",
        "layout",
        "bar",
        "treeActions",
        "culling",
        "messages",
    ] {
        assert!(
            spec[key].as_str().is_some_and(|text| !text.is_empty()),
            "assemblyView.{key} is described"
        );
    }
}
