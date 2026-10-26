// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde_json::Value;

use super::desktop as d;
use super::sound_metrics as sound;
use super::tokens::Weight;
use crate::widgets::text::{caps, line, plain};
use crate::{Language, SoundsText, Texts};

const DESKTOP: &str = include_str!("../../../../docs/design/p4kx-desktop-spec.json");
const LANGUAGES: [Language; 2] = [Language::German, Language::BritishEnglish];
const LONG_NAME: &str = "Play_SC_Ship_Weapon_Ballistic_Fire_Heavy";

fn tables() -> Value {
    let spec: Value =
        serde_json::from_str(DESKTOP).unwrap_or_else(|error| panic!("the desktop spec: {error}"));
    spec["soundTables"].clone()
}

fn widest(bodies: &[String], heads: &[String]) -> f32 {
    let mut width = 0.0_f32;
    let mut harness = egui_kittest::Harness::builder()
        .with_context_setup(crate::theme::apply)
        .build_ui(|ui| {
            for text in bodies {
                for weight in [Weight::Regular, Weight::Bold] {
                    let galley = line(ui, plain(text.as_str(), d::ROW_FONT, weight), f32::INFINITY);
                    width = width.max(galley.size().x);
                }
            }
            for head in heads {
                let galley = line(
                    ui,
                    caps(
                        head,
                        d::TABLE_HEAD_SIZE,
                        Weight::Bold,
                        d::TABLE_HEAD_TRACKING,
                    ),
                    f32::INFINITY,
                );
                width = width.max(galley.size().x);
            }
        });
    harness.run();
    drop(harness);
    width
}

fn needed(texts: &[(Language, Vec<String>, Vec<String>)]) -> f32 {
    let pad = 2.0 * d::CELL_PADDING_X;
    texts
        .iter()
        .map(|(_, bodies, heads)| widest(bodies, heads) + pad)
        .fold(0.0, f32::max)
}

fn state_texts() -> Vec<(Language, Vec<String>, Vec<String>)> {
    LANGUAGES
        .iter()
        .map(|&language| {
            let texts = Texts::new(language);
            let bodies = [
                SoundsText::StateOk,
                SoundsText::StateNoDefinition,
                SoundsText::StateNoEvent,
                SoundsText::StateNoSoundbank,
            ]
            .map(|text| texts.get(text))
            .to_vec();
            (language, bodies, vec![texts.get(SoundsText::ColumnState)])
        })
        .collect()
}

fn language_texts() -> Vec<(Language, Vec<String>, Vec<String>)> {
    LANGUAGES
        .iter()
        .map(|&language| {
            let texts = Texts::new(language);
            let fallback = texts.format(
                SoundsText::LanguageFallback,
                &[("language", "English(US)".to_owned().into())],
            );
            (
                language,
                vec![fallback],
                vec![texts.get(SoundsText::ColumnLanguage)],
            )
        })
        .collect()
}

#[test]
fn the_sound_table_columns_match_the_desktop_spec() {
    let tables = tables();
    let columns = &tables["columns"];

    for (key, value) in [
        ("name", sound::SOUND_NAME_COLUMN),
        ("state", sound::SOUND_STATE_COLUMN),
        ("language", sound::SOUND_LANGUAGE_COLUMN),
    ] {
        assert_eq!(columns[key].as_f64(), Some(f64::from(value)), "{key}");
    }
    assert_eq!(
        tables["columnMin"].as_f64(),
        Some(f64::from(sound::SOUND_COLUMN_MIN)),
        "column minimum"
    );
}

#[test]
fn the_status_column_holds_the_longest_status_and_head_in_both_languages() {
    let required = needed(&state_texts());

    assert!(
        sound::SOUND_STATE_COLUMN >= required,
        "status {required} does not fit"
    );
    assert!(
        sound::SOUND_STATE_COLUMN - required < 1.5,
        "status {required} is not rounded up"
    );
}

#[test]
fn the_language_column_holds_the_default_language_with_its_mark_in_both_languages() {
    let required = needed(&language_texts());

    assert!(
        sound::SOUND_LANGUAGE_COLUMN >= required,
        "language {required} does not fit"
    );
    assert!(
        sound::SOUND_LANGUAGE_COLUMN - required < 1.5,
        "language {required} is not rounded up"
    );
}

#[test]
fn the_name_column_holds_a_name_of_forty_characters() {
    let required = needed(&[(
        Language::BritishEnglish,
        vec![LONG_NAME.to_owned()],
        Vec::new(),
    )]);

    assert_eq!(LONG_NAME.len(), 40, "the sample name");
    assert!(
        sound::SOUND_NAME_COLUMN >= required,
        "name {required} does not fit"
    );
}
