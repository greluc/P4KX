use serde_json::Value;

use super::desktop as d;
use super::tokens::{FS_2XS, FS_XS, TRACKING_OVERLINE};

const DESKTOP: &str = include_str!("../../../../docs/design/p4kx-desktop-spec.json");

fn spec(key: &str) -> Value {
    let spec: Value =
        serde_json::from_str(DESKTOP).unwrap_or_else(|error| panic!("the desktop spec: {error}"));
    spec[key].clone()
}

fn assert_metrics(section: &str, metrics: &[(&str, f32)]) {
    let spec = spec(section);
    for &(key, value) in metrics {
        let expected = spec[key].as_f64();
        assert!(
            expected.is_some_and(|want| (want - f64::from(value)).abs() < 0.01),
            "{section}.{key} is {expected:?}, the theme has {value}"
        );
    }
}

#[test]
fn the_audio_preview_metrics_match_the_desktop_spec() {
    assert_metrics(
        "audioPreview",
        &[
            ("paddingX", d::PREVIEW_PADDING_X),
            ("paddingY", d::IMAGE_PADDING_Y),
            ("controlGap", d::AUDIO_CONTROL_GAP),
            ("volumeWidth", d::SLIDER_WIDTH),
            ("positionMinWidth", d::SLIDER_WIDTH),
            ("timeFontSize", FS_XS),
            ("labelFontSize", FS_2XS),
            ("labelLetterSpacingEm", TRACKING_OVERLINE),
        ],
    );
}

#[test]
fn the_slider_is_the_quality_row_range_in_a_compact_row() {
    assert_metrics(
        "slider",
        &[
            ("width", d::SLIDER_WIDTH),
            ("height", d::SLIDER_HEIGHT),
            ("labelGap", d::SLIDER_LABEL_GAP),
            ("hitHeight", d::SLIDER_HIT_HEIGHT),
        ],
    );
    let slider = spec("slider");

    assert_eq!(
        slider["fill"].as_str(),
        Some("color-primary"),
        "accent fill"
    );
    assert_eq!(slider["track"].as_str(), Some("progressBar"), "track");
    assert_eq!(slider["thumb"]["shape"].as_str(), Some("square"), "thumb");
    assert_eq!(
        slider["thumb"]["size"].as_f64(),
        Some(f64::from(d::SLIDER_HEIGHT)),
        "thumb size"
    );
    assert_eq!(
        slider["focusRing"]["offset"].as_f64(),
        Some(f64::from(d::FOCUS_OFFSET)),
        "focus ring"
    );
}
