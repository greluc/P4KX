// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde_json::Value;

use super::desktop as d;
use super::sound_metrics as sound;
use super::tokens::{FS_2XS, FS_XS, TRACKING_OVERLINE, Weight};
use crate::widgets::text::{line, plain};

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
    assert_eq!(slider["thumb"]["shape"].as_str(), Some("circle"), "thumb");
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

#[test]
fn the_soundbank_list_columns_match_the_desktop_spec() {
    let list = spec("soundbankList");
    let columns = &list["columns"];

    assert_eq!(
        columns["id"].as_f64(),
        Some(f64::from(sound::SOUND_ID_COLUMN)),
        "id"
    );
    assert_eq!(
        columns["codec"].as_f64(),
        Some(f64::from(sound::SOUND_CODEC_COLUMN)),
        "codec"
    );
    assert_eq!(
        columns["duration"].as_f64(),
        Some(f64::from(sound::SOUND_DURATION_COLUMN)),
        "duration"
    );
    assert_eq!(
        list["panelHeight"].as_f64(),
        Some(f64::from(sound::PREVIEW_SOUNDBANK_HEIGHT)),
        "panel height"
    );
    assert_eq!(
        columns_min(&list),
        Some(f64::from(sound::SOUND_COLUMN_MIN)),
        "column minimum"
    );
    assert_eq!(
        list["visibleRows"].as_f64(),
        Some(f64::from(sound::SOUNDBANK_VISIBLE_ROWS)),
        "visible rows"
    );
    assert_eq!(
        list["rowHeight"].as_f64(),
        Some(f64::from(d::ROW_HEIGHT)),
        "row height"
    );
}

fn columns_min(list: &Value) -> Option<f64> {
    list["columnMin"].as_f64()
}

fn widest(texts: &[&str]) -> f32 {
    let mut width = 0.0_f32;
    let mut harness = egui_kittest::Harness::builder()
        .with_context_setup(crate::theme::apply)
        .build_ui(|ui| {
            for text in texts {
                for weight in [Weight::Regular, Weight::Bold] {
                    let galley = line(ui, plain(*text, d::ROW_FONT, weight), f32::INFINITY);
                    width = width.max(galley.size().x);
                }
            }
        });
    harness.run();
    drop(harness);
    width
}

#[test]
fn the_soundbank_columns_hold_their_widest_text() {
    let pad = 2.0 * d::CELL_PADDING_X;
    let id = widest(&["4294967295", "3333333333", "2222222222", "1000000000"]) + pad;
    let codec = widest(&[
        "Wwise Vorbis",
        "IMA ADPCM",
        "DSP ADPCM",
        "Vorgeladen",
        "Prefetched",
        "Plug-in",
    ]) + pad;
    let duration = widest(&["9:59:59.999", "0:00:00.000", "8:88:88.888"]) + pad;

    assert!(sound::SOUND_ID_COLUMN >= id, "id {id}");
    assert!(
        sound::SOUND_ID_COLUMN - id < 1.0,
        "id {id} is not rounded up"
    );
    assert!(sound::SOUND_CODEC_COLUMN >= codec, "codec {codec}");
    assert!(
        sound::SOUND_CODEC_COLUMN - codec < 1.5,
        "codec {codec} is not rounded up"
    );
    assert!(
        sound::SOUND_DURATION_COLUMN >= duration,
        "duration {duration}"
    );
    assert!(
        sound::SOUND_DURATION_COLUMN - duration < 1.0,
        "duration {duration} is not rounded up"
    );
}
