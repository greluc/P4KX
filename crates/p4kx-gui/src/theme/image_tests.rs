use serde_json::Value;

use super::desktop as d;
use super::tokens::{FS_2XS, TRACKING_OVERLINE};

const DESKTOP: &str = include_str!("../../../../docs/design/p4kx-desktop-spec.json");

fn image_preview() -> Value {
    let spec: Value =
        serde_json::from_str(DESKTOP).unwrap_or_else(|error| panic!("the desktop spec: {error}"));
    spec["imagePreview"].clone()
}

#[test]
fn the_image_preview_metrics_match_the_desktop_spec() {
    let spec = image_preview();

    for (key, value) in [
        ("paddingX", d::IMAGE_PADDING_X),
        ("paddingY", d::IMAGE_PADDING_Y),
        ("labelGap", d::IMAGE_LABEL_GAP),
        ("selectorGap", d::IMAGE_SELECTOR_GAP),
        ("labelFontSize", FS_2XS),
        ("labelLetterSpacingEm", TRACKING_OVERLINE),
        ("focusRingOffset", d::SEGMENT_FOCUS_OFFSET),
    ] {
        let expected = spec[key].as_f64();
        assert!(
            expected.is_some_and(|want| (want - f64::from(value)).abs() < 0.01),
            "imagePreview.{key} is {expected:?}, the theme has {value}"
        );
    }
}

#[test]
fn the_image_preview_names_its_classes_and_surface() {
    let spec = image_preview();

    assert_eq!(
        spec["dsClass"].as_str(),
        Some("previewPanel + facts-bar + krt-segmented krt-segmented--sm"),
        "classes"
    );
    assert_eq!(
        spec["background"].as_str(),
        Some("surface-panel"),
        "surface"
    );
    assert_eq!(
        spec["labelFontWeight"].as_str(),
        Some("fw-bold"),
        "label weight"
    );
    assert_eq!(spec["labelUppercase"].as_bool(), Some(true), "label case");
    assert_eq!(
        spec["labelColor"].as_str(),
        Some("color-gray-2-text"),
        "label colour"
    );
}
