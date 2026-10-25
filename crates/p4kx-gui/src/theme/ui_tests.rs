// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde_json::Value;

use super::ui_metrics as m;

const DESKTOP: &str = include_str!("../../../../docs/design/p4kx-desktop-spec.json");

fn spec() -> Value {
    serde_json::from_str(DESKTOP).unwrap_or_else(|error| panic!("the desktop spec: {error}"))
}

#[test]
fn the_ui_tab_metrics_match_the_desktop_spec() {
    let json = spec();

    for (key, value) in [
        ("rowHeight", m::UI_ROW_HEIGHT),
        ("pictureMaxScale", m::UI_PICTURE_MAX_SCALE),
    ] {
        let expected = json["uiTab"][key].as_f64();
        assert!(
            expected.is_some_and(|want| (want - f64::from(value)).abs() < 0.01),
            "uiTab.{key} is {expected:?}, the theme has {value}"
        );
    }
    assert_eq!(
        json["uiTab"]["reportMaxLines"].as_u64(),
        Some(m::UI_REPORT_MAX_LINES as u64),
        "uiTab.reportMaxLines"
    );
    assert_eq!(json["uiTab"]["decision"], "D.51", "the decision");
}
