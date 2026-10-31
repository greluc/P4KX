// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align2, Frame, Id, Margin, TextEdit, Ui, vec2};
use fluent_bundle::FluentValue;
use p4kx_settings::MAX_VARIABLES_BYTES;

use super::search::eyebrow;
use crate::theme::desktop::{SEARCH_WIDTH, TOOLBAR_PADDING};
use crate::theme::palette;
use crate::theme::tokens::{FS_BODY, FS_SM, TOUCH_TARGET, Weight, font};
use crate::widgets::field;
use crate::widgets::frames::px;
use crate::widgets::text::plain;
use crate::{Model, UiText};

/// Shows the field for more game variables of a screen, and the settings in it that are no
/// `NAME=VALUE`; returns whether the field was left, so that the text is stored.
pub(super) fn show(ui: &mut Ui, model: &mut Model) -> bool {
    let texts = model.texts();
    let label = eyebrow(&texts.get(UiText::VariablesLabel));
    let hint = texts.get(UiText::VariablesHint);
    let invalid = model.ui().invalid_variables();
    let warning = (!invalid.is_empty()).then(|| {
        texts.format(
            UiText::VariablesInvalid,
            &[("settings", FluentValue::from(invalid.join(", ")))],
        )
    });
    let mut text = model.ui().variables_text().to_owned();
    let mut edited = Edited::default();
    ui.horizontal(|ui| {
        let label = ui.label(label);
        edited = edit(ui, &mut text, &hint, label.id);
    });
    if let Some(warning) = warning {
        ui.label(plain(warning, FS_SM, Weight::Regular).color(palette::WARNING));
    }
    if edited.changed {
        model.ui_mut().set_variables_text(&text);
    }
    edited.left
}

/// What happened to the field in this frame.
#[derive(Debug, Default, Clone, Copy)]
struct Edited {
    changed: bool,
    left: bool,
}

fn edit(ui: &mut Ui, text: &mut String, hint: &str, label: Id) -> Edited {
    let bloom = field::bloom_slot(ui);
    let frame = field::frame_slot(ui);
    let edit = TextEdit::singleline(text)
        .hint_text(plain(hint, FS_BODY, Weight::Regular).italics())
        .font(font(FS_BODY, Weight::Regular))
        .frame(Frame::NONE.inner_margin(Margin {
            left: px(TOOLBAR_PADDING),
            right: px(TOOLBAR_PADDING),
            top: 0,
            bottom: 0,
        }))
        .min_size(vec2(SEARCH_WIDTH.min(ui.available_width()), TOUCH_TARGET))
        .desired_width(ui.available_width())
        .char_limit(MAX_VARIABLES_BYTES)
        .align(Align2::LEFT_CENTER);
    let response = ui.add(edit).labelled_by(label);
    field::paint_frame(ui, frame, response.rect, response.has_focus());
    if response.has_focus() {
        field::paint_bloom(ui, bloom, response.rect);
    }
    Edited {
        changed: response.changed(),
        left: response.lost_focus(),
    }
}
