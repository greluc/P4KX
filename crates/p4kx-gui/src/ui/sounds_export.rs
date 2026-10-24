// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align, Layout, Ui};
use fluent_bundle::FluentValue;

use super::extraction::{POLICIES, folder_field, foot, group_label, number, refusal_notice};
use super::{Dialog, Requests};
use crate::theme::desktop::MODAL_GROUP_GAP;
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, Tone};
use crate::widgets::toggle;
use crate::{
    ExtractionText, Model, SoundFormat, SoundsExportSetup, SoundsScope, SoundsText as T, Text,
    Texts,
};

const FORMATS: [(SoundFormat, ExtractionText); 3] = [
    (SoundFormat::Ogg, ExtractionText::AudioOgg),
    (SoundFormat::Wav, ExtractionText::AudioWav),
    (SoundFormat::Raw, ExtractionText::AudioRaw),
];

enum Action {
    Submit,
    Close,
    ChooseFolder,
}

/// Shows the setup of a sound export while it is open and applies what the user chose.
pub(super) fn setup(ui: &Ui, model: &mut Model, requests: &mut Requests) {
    let Some((setup, texts)) = model.sounds_export_setup_with_texts() else {
        return;
    };
    let title = texts.get(T::ExportTitle);
    let close_dialog = texts.get(Text::CloseDialog);
    let shown =
        frames::modal(frames::modal_id("sounds-export-setup"), Tone::Normal).show(ui.ctx(), |ui| {
            ui.set_width(frames::MODAL_WIDE_BODY_WIDTH);
            let closed = frames::head(ui, (&title, Tone::Normal), true, Some(&close_dialog));
            let action = choices(ui, texts, setup);
            if closed { Some(Action::Close) } else { action }
        });
    frames::modal_decoration(
        ui.ctx(),
        shown.response.layer_id,
        shown.response.rect,
        Tone::Normal,
    );
    let close = shown.should_close();
    match shown.inner {
        Some(Action::Submit) => {
            requests.settings_changed |= model.submit_sounds_export();
        }
        Some(Action::Close) => model.sounds_mut().close_export_setup(),
        Some(Action::ChooseFolder) => requests.dialog = Some(Dialog::SoundsFolder),
        None if close => model.sounds_mut().close_export_setup(),
        None => {}
    }
}

#[expect(clippy::too_many_lines, reason = "one block per control of the form")]
fn choices(ui: &mut Ui, texts: &Texts, setup: &mut SoundsExportSetup) -> Option<Action> {
    let mut action = None;
    group_label(ui, &texts.get(T::ExportScope));
    let selected = setup.selected().map_or_else(
        || texts.get(T::ExportSelectedNone),
        |name| texts.format(T::ExportSelected, &[("name", name.to_owned().into())]),
    );
    let has_selection = setup.selected().is_some();
    if toggle::radio(
        ui,
        setup.scope() == SoundsScope::Selected,
        &selected,
        has_selection,
    )
    .clicked()
    {
        setup.set_scope(SoundsScope::Selected);
    }
    let all = texts.format(
        T::ExportAll,
        &[
            (
                "count",
                FluentValue::from(u64::try_from(setup.all_len()).unwrap_or(u64::MAX)),
            ),
            ("number", number(texts, setup.all_len())),
        ],
    );
    if toggle::radio(
        ui,
        setup.scope() == SoundsScope::All,
        &all,
        setup.all_len() > 0,
    )
    .clicked()
    {
        setup.set_scope(SoundsScope::All);
    }
    ui.add_space(MODAL_GROUP_GAP);
    group_label(ui, &texts.get(T::ExportFormat));
    for (format, text) in FORMATS {
        if toggle::radio(ui, setup.format() == format, &texts.get(text), true).clicked() {
            setup.set_format(format);
        }
    }
    ui.add_space(MODAL_GROUP_GAP);
    group_label(ui, &texts.get(ExtractionText::TargetHeading));
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let choose = texts.get(ExtractionText::ChooseFolder);
            if button::button(ui, Kind::Ghost, Size::Normal, &choose, true).clicked() {
                action = Some(Action::ChooseFolder);
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                folder_field(ui, texts, setup.folder());
            });
        });
    });
    refusal_notice(ui, texts, setup.refusal());
    ui.add_space(MODAL_GROUP_GAP);
    group_label(ui, &texts.get(ExtractionText::PolicyHeading));
    for (policy, text) in POLICIES {
        if toggle::radio(ui, setup.policy() == policy, &texts.get(text), true).clicked() {
            setup.set_policy(policy);
        }
    }
    let pressed = foot(
        ui,
        texts,
        (T::ExportStart, setup.is_ready()),
        ExtractionText::Close,
    );
    action.or(match pressed {
        Some(true) => Some(Action::Submit),
        Some(false) => Some(Action::Close),
        None => None,
    })
}
