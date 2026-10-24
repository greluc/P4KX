// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde_json::Value;

use super::desktop as d;

const DESKTOP: &str = include_str!("../../../../docs/design/p4kx-desktop-spec.json");

fn xml_preview() -> Value {
    let spec: Value =
        serde_json::from_str(DESKTOP).unwrap_or_else(|error| panic!("the desktop spec: {error}"));
    spec["xmlPreview"].clone()
}

#[test]
fn the_xml_preview_metrics_match_the_desktop_spec() {
    let spec = xml_preview();

    for (key, value) in [
        ("rowHeight", d::XML_ROW_HEIGHT),
        ("fontSize", d::XML_FONT),
        ("gutterPaddingX", d::XML_GUTTER_PADDING_X),
        ("focusRingOffset", -d::FOCUS_WIDTH),
    ] {
        let expected = spec[key].as_f64();
        assert!(
            expected.is_some_and(|want| (want - f64::from(value)).abs() < 0.01),
            "xmlPreview.{key} is {expected:?}, the theme has {value}"
        );
    }
}

#[test]
fn the_xml_preview_names_its_classes_and_colours() {
    let spec = xml_preview();

    for (key, expected) in [
        ("dsClass", "previewPanel + hexRow"),
        ("fontWeight", "fw-regular"),
        ("textColor", "color-gray-1"),
        ("lineNumberColor", "color-gray-2-text"),
        ("escapeColor", "color-gray-2-text"),
        ("background", "surface-panel"),
    ] {
        assert_eq!(spec[key].as_str(), Some(expected), "xmlPreview.{key}");
    }
    assert_eq!(
        spec["tabular"].as_bool(),
        Some(false),
        "Lato figures, ADR-0006"
    );
}
