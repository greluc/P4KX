// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde_json::Value;

use super::character_metrics as m;

const DESKTOP: &str = include_str!("../../../../docs/design/p4kx-desktop-spec.json");

fn character_builder() -> Value {
    let spec: Value =
        serde_json::from_str(DESKTOP).unwrap_or_else(|error| panic!("the desktop spec: {error}"));
    spec["characterBuilder"].clone()
}

#[test]
fn the_character_builder_metrics_match_the_desktop_spec() {
    let spec = character_builder();

    for (key, value) in [
        ("stackWidth", m::CHARACTER_STACK_WIDTH),
        ("reportRows", m::CHARACTER_REPORT_ROWS),
        ("reportHeight", m::CHARACTER_REPORT_HEIGHT),
        ("expressionRows", m::CHARACTER_EXPRESSION_ROWS),
    ] {
        let expected = spec[key].as_f64();
        assert!(
            expected.is_some_and(|want| (want - f64::from(value)).abs() < 0.01),
            "characterBuilder.{key} is {expected:?}, the theme has {value}"
        );
    }
}

#[test]
fn the_character_builder_names_its_decision_and_parts() {
    let spec = character_builder();

    assert_eq!(spec["decision"].as_str(), Some("D.48"), "decision");
    for key in ["dsClass", "layout", "choices", "report", "messages"] {
        assert!(
            spec[key].as_str().is_some_and(|text| !text.is_empty()),
            "characterBuilder.{key} is described"
        );
    }
}
