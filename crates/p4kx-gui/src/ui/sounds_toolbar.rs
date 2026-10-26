// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align, Key, Layout, Ui, vec2};

use super::search::{eyebrow, field, field_response};
use crate::theme::desktop::TOOLBAR_PADDING;
use crate::widgets::button::{self, Kind, Size};
use crate::{ListStatus, Model, SoundsStatus, SoundsText};

enum Action {
    Show,
    UseRecord,
    Export,
    Language(Pick),
}

enum Pick {
    Default,
    Named(String),
}

/// Shows the Entity and search fields with their actions and applies what the user did.
#[expect(clippy::too_many_lines, reason = "one block per control of the form")]
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
    let texts = model.texts();
    let entity_label = eyebrow(&texts.get(SoundsText::EntityLabel));
    let entity_hint = texts.get(SoundsText::EntityHint);
    let search_label = eyebrow(&texts.get(SoundsText::SearchLabel));
    let search_hint = texts.get(SoundsText::SearchHint);
    let show_label = texts.get(SoundsText::Show);
    let record_label = texts.get(SoundsText::UseRecord);
    let export_label = texts.get(SoundsText::Export);
    let tab = model.sounds();
    let mut entity = tab.entity_text().to_owned();
    let mut query = tab.query().to_owned();
    let can_export = matches!(
        tab.list_status(),
        ListStatus::Ready(list) if list.rows.iter().any(|row| row.unmatched.is_none())
    );
    let has_record = model.has_selected_record();
    let mut action = None;
    let mut entity_changed = false;
    ui.horizontal(|ui| {
        let label = ui.label(entity_label);
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            let response = field_response(ui, &mut entity, &entity_hint, label.id);
            entity_changed |= response.changed();
            if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                action = Some(Action::Show);
            }
        });
    });
    ui.horizontal_wrapped(|ui| {
        let ready = !entity.trim().is_empty();
        if button::button(ui, Kind::Outline, Size::Normal, &show_label, ready).clicked() {
            action = Some(Action::Show);
        }
        if button::button(ui, Kind::Ghost, Size::Normal, &record_label, has_record).clicked() {
            action = Some(Action::UseRecord);
        }
        if button::button(ui, Kind::Ghost, Size::Normal, &export_label, can_export).clicked() {
            action = Some(Action::Export);
        }
    });
    if let Some(pick) = languages(ui, model) {
        action = Some(Action::Language(pick));
    }
    let mut query_changed = false;
    ui.horizontal(|ui| {
        let label = ui.label(search_label);
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            query_changed = field(ui, &mut query, &search_hint, label.id);
        });
    });
    if entity_changed || matches!(action, Some(Action::Show)) {
        model.sounds_mut().set_entity_text(&entity);
    }
    if query_changed {
        model.sounds_mut().set_query(&query);
    }
    match action {
        Some(Action::Show) => {
            model.sounds_mut().show_entity_text();
        }
        Some(Action::UseRecord) => {
            model.show_selected_record_sounds();
        }
        Some(Action::Export) => {
            model.begin_sounds_export();
        }
        Some(Action::Language(pick)) => {
            let name = match &pick {
                Pick::Default => None,
                Pick::Named(name) => Some(name.as_str()),
            };
            model.sounds_mut().set_language(name);
        }
        None => {}
    }
}

/// Shows the label and one button per language folder of the Archive, the language the sounds are
/// resolved in marked; returns what the user chose.
fn languages(ui: &mut Ui, model: &Model) -> Option<Pick> {
    let SoundsStatus::Ready(info) = model.sounds().status() else {
        return None;
    };
    if info.languages.len() < 2 {
        return None;
    }
    let current = model
        .sounds()
        .language()
        .or(info.default_language.as_deref());
    let label = eyebrow(&model.texts().get(SoundsText::LanguageLabel));
    ui.horizontal_wrapped(|ui| {
        ui.label(label);
        let mut chosen = None;
        for name in &info.languages {
            let active = current == Some(name.as_str());
            if button::choice(ui, Size::Compact, name, active).clicked() {
                let is_default = info.default_language.as_deref() == Some(name.as_str());
                chosen = Some(if is_default {
                    Pick::Default
                } else {
                    Pick::Named(name.clone())
                });
            }
        }
        chosen
    })
    .inner
}
