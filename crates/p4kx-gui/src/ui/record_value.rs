// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::Color32;
use fluent_bundle::FluentValue;

use crate::theme::palette;
use crate::{KeyText, LineValue, RecordChangeText, RecordLanguageText, Texts, format};

/// Returns the value of `line` as the Record preview shows it, with its colour: a Reference in
/// the link colour, an error value in the danger tint with its reason.
pub(super) fn value_text(texts: &Texts, value: &LineValue) -> (String, Color32) {
    let error = |reason: &str| {
        let reason = ("reason", FluentValue::from(reason.to_owned()));
        (
            texts.format(RecordChangeText::ValueError, &[reason]),
            palette::DANGER_TEXT,
        )
    };
    match value {
        LineValue::Scalar(text) => (text.to_string(), palette::GRAY_1),
        LineValue::Key { key, text } => (
            key.to_string(),
            if matches!(text, KeyText::None) {
                palette::GRAY_1
            } else {
                palette::GRAY_2_TEXT
            },
        ),
        LineValue::Struct(type_name) => (
            texts.format(
                RecordChangeText::ValueStruct,
                &[("type", type_name.to_string().into())],
            ),
            palette::GRAY_1,
        ),
        LineValue::Array(len) => {
            let count = u64::try_from(*len).unwrap_or(u64::MAX);
            let args = [
                ("count", FluentValue::from(count)),
                ("number", format::number(count, texts.language()).into()),
            ];
            (
                texts.format(RecordChangeText::ValueArray, &args),
                palette::GRAY_1,
            )
        }
        LineValue::Null => (texts.get(RecordChangeText::ValueNull), palette::GRAY_2_TEXT),
        LineValue::Reference(link) => {
            let text = link.at().map_or_else(
                || link.record_path().to_owned(),
                |at| {
                    texts.format(
                        RecordChangeText::ValueReferenceAt,
                        &[
                            ("path", link.record_path().to_owned().into()),
                            ("at", at.to_owned().into()),
                        ],
                    )
                },
            );
            (text, palette::PRIMARY)
        }
        LineValue::Dangling { reason, .. } | LineValue::Error(reason) => error(reason),
    }
}

/// Returns the value of `line` for assistive technology and the tooltip: a localisation key with
/// its text, or the mark that it has none.
pub(super) fn spoken_value(texts: &Texts, value: &LineValue) -> String {
    match value {
        LineValue::Key {
            key,
            text: KeyText::Text { text, .. },
        } => texts.format(
            RecordLanguageText::KeyRow,
            &[
                ("key", key.to_string().into()),
                ("text", text.to_string().into()),
            ],
        ),
        LineValue::Key {
            key,
            text: KeyText::Missing,
        } => texts.format(
            RecordLanguageText::KeyRowMissing,
            &[("key", key.to_string().into())],
        ),
        _ => value_text(texts, value).0,
    }
}
