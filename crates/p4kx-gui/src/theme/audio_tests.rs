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
            ("volumeWidth", d::AUDIO_VOLUME_WIDTH),
            ("positionMinWidth", d::AUDIO_POSITION_MIN_WIDTH),
            ("timeFontSize", FS_XS),
            ("labelFontSize", FS_2XS),
            ("labelLetterSpacingEm", TRACKING_OVERLINE),
        ],
    );
}

#[test]
fn the_slider_is_the_progress_bar_in_a_compact_row() {
    assert_metrics("slider", &[("hitHeight", d::SLIDER_HIT_HEIGHT)]);
    assert_metrics("progressBar", &[("height", d::PROGRESS_HEIGHT)]);
    let slider = spec("slider");

    assert_eq!(
        slider["dsClass"].as_str(),
        Some("attendance-meter"),
        "class"
    );
    assert_eq!(
        slider["borderHover"].as_str(),
        Some("color-primary"),
        "hover border"
    );
    assert_eq!(
        slider["focusRing"]["offset"].as_f64(),
        Some(f64::from(d::FOCUS_OFFSET)),
        "focus ring"
    );
}
