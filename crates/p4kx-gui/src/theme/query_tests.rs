use egui::Color32;
use serde_json::Value;

use super::desktop as d;
use super::tokens::{FS_BODY, FS_SM, FS_XS, TOUCH_TARGET};

const DESKTOP: &str = include_str!("../../../../docs/design/p4kx-desktop-spec.json");

const METRICS: [(&str, &str, f32); 17] = [
    ("queryEditor", "paddingX", d::TEXTAREA_PADDING_X),
    ("queryEditor", "paddingY", d::TEXTAREA_PADDING_Y),
    ("queryEditor", "fontSize", FS_BODY),
    ("queryEditor", "lineHeight", d::TEXTAREA_LINE_HEIGHT),
    ("errorAlert", "paddingX", super::tokens::SPACE_4),
    ("errorAlert", "paddingY", d::NOTICE_PADDING_Y),
    ("errorAlert", "fontSize", FS_SM),
    ("pagination", "gap", d::PAGE_GAP),
    ("pagination", "buttonSize", TOUCH_TARGET),
    ("pagination", "buttonPaddingX", d::PAGE_BUTTON_PADDING_X),
    ("pagination", "disabledOpacity", d::PAGE_DISABLED_OPACITY),
    ("pagination", "statusPaddingX", d::PAGE_STATUS_PADDING_X),
    ("pagination", "statusFontSize", FS_XS),
    ("pagination", "chevron", d::CHEVRON),
    ("queryTable", "columnWidth", d::QUERY_COLUMN_WIDTH),
    ("queryTable", "columnMinWidth", d::QUERY_COLUMN_MIN_WIDTH),
    ("errorAlert", "borderLeft", d::ACCENT_BAR),
];

fn spec() -> Value {
    serde_json::from_str(DESKTOP).unwrap_or_else(|error| panic!("the desktop spec: {error}"))
}

fn number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value["width"].as_f64())
        .or_else(|| value["size"].as_f64())
}

#[test]
fn the_query_metrics_match_the_desktop_spec() {
    let json = spec();

    for (element, key, value) in METRICS {
        let expected = number(&json[element][key]);
        assert!(
            expected.is_some_and(|want| (want - f64::from(value)).abs() < 0.01),
            "{element}.{key} is {expected:?}, the theme has {value}"
        );
    }
}

#[test]
fn the_query_counts_match_the_desktop_spec() {
    let json = spec();

    assert_eq!(
        json["queryEditor"]["rows"].as_u64(),
        u64::try_from(d::QUERY_EDITOR_ROWS).ok(),
        "editor rows"
    );
    assert_eq!(
        json["queryTable"]["pageRows"].as_u64(),
        u64::try_from(crate::QUERY_PAGE_ROWS).ok(),
        "page rows"
    );
}

#[test]
fn the_danger_alert_takes_the_fill_of_the_design_system() {
    let json = spec();

    assert_eq!(
        json["errorAlert"]["background"].as_str(),
        Some("rgba(163,0,10,0.2)"),
        "spec"
    );
    assert_eq!(
        d::ALERT_DANGER_FILL,
        Color32::from_rgba_unmultiplied(163, 0, 10, 51),
        "0.2 of 255"
    );
}
