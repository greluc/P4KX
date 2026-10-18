use egui::Color32;
use serde_json::Value;

use super::desktop as d;

const DESKTOP: &str = include_str!("../../../../docs/design/p4kx-desktop-spec.json");

const METRICS: [(&str, &str, f32); 23] = [
    ("archivePicker", "height", d::BUTTON_HEIGHT),
    ("archivePicker", "paddingRight", d::PICKER_PADDING_RIGHT),
    ("archivePicker", "fontSize", super::tokens::FS_BODY),
    (
        "comparisonProgress",
        "maxWidth",
        d::COMPARISON_PROGRESS_WIDTH,
    ),
    ("archivePicker", "chevron", d::CHEVRON),
    ("archivePicker", "paddingX", d::PICKER_PADDING_X),
    ("archivePicker", "minWidth", d::PICKER_MIN_WIDTH),
    ("factsBar", "paddingX", d::FACTS_PADDING_X),
    ("factsBar", "paddingY", d::FACTS_PADDING_Y),
    ("factsBar", "gap", d::FACTS_GAP),
    ("factsBar", "keyFontSize", d::FACT_KEY_SIZE),
    ("factsBar", "keyLetterSpacingEm", d::FACT_KEY_TRACKING),
    ("factsBar", "keyGap", d::FACT_KEY_GAP),
    ("factsBar", "valueFontSize", d::FACT_VALUE_SIZE),
    ("kpiTile", "paddingX", d::KPI_PADDING_X),
    ("kpiTile", "paddingY", d::KPI_PADDING_Y),
    ("kpiTile", "gap", d::TOOLBAR_PADDING),
    ("kpiTile", "valueFontSize", d::KPI_VALUE_SIZE),
    ("changeChip", "paddingX", d::CHIP_PADDING_X),
    ("changeChip", "paddingY", d::CHIP_PADDING_Y),
    ("changeChip", "letterSpacingEm", d::CHIP_TRACKING),
    ("changeChip", "fontSize", super::tokens::FS_2XS),
    ("kpiTile", "borderLeft", d::ACCENT_BAR),
];

const COLUMNS: [(&str, f32); 4] = [
    ("change", d::CHANGE_COLUMN),
    ("oldSize", d::CHANGE_SIZE_COLUMN),
    ("newSize", d::CHANGE_SIZE_COLUMN),
    ("count", d::COUNT_COLUMN),
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

fn rgba(text: &str) -> Option<Color32> {
    let inner = text.strip_prefix("rgba(")?.strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    let [r, g, b, a] = parts.as_slice() else {
        return None;
    };
    let alpha: f32 = a.parse().ok()?;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "an alpha between 0 and 1 scales into a byte"
    )]
    let alpha = (alpha * 255.0).round() as u8;
    Some(Color32::from_rgba_unmultiplied(
        r.parse().ok()?,
        g.parse().ok()?,
        b.parse().ok()?,
        alpha,
    ))
}

#[test]
fn the_comparison_metrics_match_the_desktop_spec() {
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
fn the_change_chips_take_the_fills_of_the_design_system() {
    let json = spec();

    for (kind, fill) in [
        ("added", d::CHIP_SUCCESS_FILL),
        ("removed", d::CHIP_DANGER_FILL),
        ("changed", d::CHIP_WARNING_FILL),
        ("moved", d::CHIP_INFO_FILL),
    ] {
        let expected = json["changeChip"][kind]["fill"].as_str().and_then(rgba);
        assert_eq!(expected, Some(fill), "{kind} fill");
    }
    assert_eq!(
        json["changeChip"]["unreadable"]["fill"].as_str(),
        Some("transparent"),
        "the muted chip has no fill"
    );
}

#[test]
fn the_record_list_columns_match_the_desktop_spec() {
    let json = spec();
    let columns = &json["recordTable"]["columns"];

    assert_eq!(
        columns["change"].as_f64(),
        Some(f64::from(d::CHANGE_COLUMN)),
        "change"
    );
    assert_eq!(
        columns["type"].as_f64(),
        Some(f64::from(d::RECORD_TYPE_COLUMN)),
        "type"
    );
}

#[test]
fn the_change_list_columns_match_the_desktop_spec() {
    let json = spec();
    let table = &json["changeTable"];

    for (column, width) in COLUMNS {
        let expected = table["columns"][column]
            .as_f64()
            .or_else(|| table["directoryColumns"][column].as_f64());
        assert_eq!(expected, Some(f64::from(width)), "{column}");
    }
}

#[test]
fn the_line_diff_metrics_and_fills_match_the_desktop_spec() {
    let json = spec();
    let diff = &json["lineDiff"];

    for (key, value) in [
        ("rowHeight", d::DIFF_ROW_HEIGHT),
        ("fontSize", d::DIFF_FONT),
        ("gutterPaddingX", d::DIFF_GUTTER_PADDING_X),
        ("markerGap", d::DIFF_MARKER_GAP),
    ] {
        let expected = number(&diff[key]);
        assert!(
            expected.is_some_and(|want| (want - f64::from(value)).abs() < 0.01),
            "lineDiff.{key} is {expected:?}, the theme has {value}"
        );
    }
    assert_eq!(
        number(&diff["hunkHeader"]["height"]),
        Some(f64::from(d::DIFF_ROW_HEIGHT)),
        "hunk header height"
    );
    for (kind, fill) in [
        ("added", d::DIFF_ADDED_FILL),
        ("removed", d::DIFF_REMOVED_FILL),
    ] {
        let expected = diff[kind]["fill"].as_str().and_then(rgba);
        assert_eq!(expected, Some(fill), "{kind} fill");
    }
}
