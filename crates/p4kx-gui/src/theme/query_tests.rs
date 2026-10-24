// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::Color32;
use serde_json::Value;

use super::desktop as d;
use super::tokens::{FS_BODY, FS_SM, FS_XS, TOUCH_TARGET};

const DESKTOP: &str = include_str!("../../../../docs/design/p4kx-desktop-spec.json");

const METRICS: [(&str, &str, f32); 28] = [
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
    ("pagination", "chevron", d::PAGE_CHEVRON),
    ("listFooter", "paddingX", d::LIST_FOOTER_PADDING_X),
    ("listFooter", "paddingY", d::LIST_FOOTER_PADDING_Y),
    ("listFooter", "fontSize", FS_SM),
    ("queryTable", "columnWidth", d::QUERY_COLUMN_WIDTH),
    ("queryTable", "columnMinWidth", d::QUERY_COLUMN_MIN_WIDTH),
    ("errorAlert", "borderLeft", d::ACCENT_BAR),
    ("queryCompletion", "maxHeight", d::COMPLETION_MAX_HEIGHT),
    ("queryCompletion", "minWidth", d::DROPDOWN_MIN_WIDTH),
    (
        "queryCompletion",
        "noticePadding",
        d::COMBOBOX_NOTICE_PADDING,
    ),
    ("savedQueries", "listHeight", d::SAVED_LIST_HEIGHT),
    ("savedQueries", "rowHeight", d::ROW_HEIGHT),
    ("savedQueries", "paddingX", d::SIDEBAR_PADDING_X),
    ("savedQueries", "gap", d::CHANNEL_GAP),
    ("savedQueries", "nameMaxWidth", d::SAVED_NAME_MAX_WIDTH),
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
        json["queryEditor"]["maxRows"].as_u64(),
        u64::try_from(d::QUERY_EDITOR_MAX_ROWS).ok(),
        "editor rows before it scrolls"
    );
    assert_eq!(
        json["queryTable"]["pageRows"].as_u64(),
        u64::try_from(crate::QUERY_PAGE_ROWS).ok(),
        "page rows"
    );
}

#[test]
fn the_danger_alert_takes_the_fill_of_the_theme() {
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

#[test]
fn the_quiet_danger_button_takes_the_tints_of_the_theme() {
    let json = spec();

    assert_eq!(
        json["buttonQuietDanger"]["hoverBackground"].as_str(),
        Some("rgba(163,0,10,0.12)"),
        "hover"
    );
    assert_eq!(
        json["buttonQuietDanger"]["pressedBackground"].as_str(),
        Some("rgba(163,0,10,0.2)"),
        "pressed"
    );
    assert_eq!(
        d::QUIET_DANGER_HOVER,
        Color32::from_rgba_unmultiplied(163, 0, 10, 31),
        "0.12 of 255"
    );
}

#[test]
fn the_danger_button_is_filled_red_after_the_theme() {
    use crate::theme::palette;
    use crate::widgets::button::{Kind, Phase, look};

    let json = spec();
    assert_eq!(
        json["buttonDanger"]["background"].as_str(),
        Some("color-danger"),
        "spec fill"
    );
    assert_eq!(
        json["buttonDanger"]["hoverBackground"].as_str(),
        Some("color-danger-hover"),
        "spec hover"
    );

    let rest = look(Kind::Danger, Phase::Rest);
    assert_eq!(
        (rest.fill, rest.text),
        (palette::DANGER, palette::WHITE),
        "rest"
    );
    assert_eq!(
        look(Kind::Danger, Phase::Hover).fill,
        palette::DANGER_HOVER,
        "hover"
    );
}
