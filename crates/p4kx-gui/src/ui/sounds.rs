// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align, Color32, Key, Label, Layout, Margin, Panel, Ui, vec2};
use fluent_bundle::FluentValue;

use super::audio_preview::{self, AudioAction, Named};
use super::record_export_outcome;
use super::search::{eyebrow, field, field_response};
use super::{RESIZE_GAP, empty, list_footer, sounds_triggers, sounds_variants};
use crate::theme::desktop::{
    PREVIEW_HEIGHT, PREVIEW_MIN_HEIGHT, SPINNER, SPINNER_INLINE, TOOLBAR_PADDING,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, px};
use crate::widgets::indicator;
use crate::widgets::text::{hairline_below, plain};
use crate::{
    IndexInfo, ListSource, ListStatus, Model, Playable, SoundList, SoundsListStatus, SoundsStatus,
    SoundsText, Texts, TriggerList, format,
};

enum Action {
    Show,
    UseRecord,
    Export,
}

/// Shows the Sounds tab: the Entity and search fields, the audio triggers they list and, below,
/// the sounds of the selected trigger with the player, or the state of indexing the audio.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    if model.browser().is_none() {
        let texts = model.texts();
        let title = texts.get(SoundsText::EmptyTitle);
        let hint = texts.get(SoundsText::EmptyHint);
        empty::notice(ui, &title, &hint);
        return;
    }
    if !model.sounds().is_started() {
        let ctx = ui.ctx().clone();
        model.load_sounds(move || ctx.request_repaint());
    }
    let texts = model.texts();
    match model.sounds().status() {
        SoundsStatus::Idle | SoundsStatus::Building => {
            frames::panel().show(ui, |ui| loading(ui, texts, SoundsText::Loading));
        }
        SoundsStatus::Failed(reason) => {
            let text = reason.map_or_else(
                || texts.get(SoundsText::Stopped),
                |reason| texts.format(SoundsText::Failed, &[("reason", reason.to_owned().into())]),
            );
            frames::panel().show(ui, |ui| {
                frames::danger_alert(ui, |ui| {
                    ui.add(Label::new(text).selectable(true));
                });
            });
        }
        SoundsStatus::ArchiveChanged => {
            let text = texts.get(SoundsText::ArchiveChanged);
            frames::panel().show(ui, |ui| {
                frames::warning_alert(ui, |ui| {
                    ui.add(Label::new(text).selectable(true));
                });
            });
        }
        SoundsStatus::Ready(info) => {
            let info = *info;
            ready(ui, model, info);
        }
    }
}

fn ready(ui: &mut Ui, model: &mut Model, info: IndexInfo) {
    let toolbar = frames::panel()
        .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
        .show(ui, |ui| toolbar(ui, model));
    hairline_below(ui, toolbar.response.rect);
    if info.issues > 0 {
        issues(ui, model.texts(), info.issues);
    }
    let action = Panel::bottom("sounds-detail")
        .frame(frames::panel().outer_margin(Margin {
            top: RESIZE_GAP,
            ..Margin::ZERO
        }))
        .resizable(true)
        .default_size(PREVIEW_HEIGHT)
        .min_size(PREVIEW_MIN_HEIGHT)
        .show(ui, |ui| detail(ui, model))
        .inner;
    let picked = frames::panel()
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            triggers(ui, model)
        })
        .inner;
    if let Some(position) = picked {
        model.sounds_mut().select(position);
    }
    match action {
        Some(DetailAction::Audio(action)) => audio_preview::apply(model, action),
        Some(DetailAction::Select(position)) => {
            let ctx = ui.ctx().clone();
            model.select_sound_variant(position, move || ctx.request_repaint());
        }
        None => {}
    }
}

fn issues(ui: &mut Ui, texts: &Texts, count: usize) {
    let count = u64::try_from(count).unwrap_or(u64::MAX);
    let text = texts.format(
        SoundsText::Issues,
        &[
            ("count", FluentValue::from(count)),
            ("number", format::number(count, texts.language()).into()),
        ],
    );
    frames::warning_alert(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.add(Label::new(text).selectable(true));
    });
}

#[expect(clippy::too_many_lines, reason = "one block per control of the form")]
fn toolbar(ui: &mut Ui, model: &mut Model) {
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
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if button::button(ui, Kind::Ghost, Size::Normal, &export_label, can_export).clicked() {
                action = Some(Action::Export);
            }
            if button::button(ui, Kind::Ghost, Size::Normal, &record_label, has_record).clicked() {
                action = Some(Action::UseRecord);
            }
            let ready = !entity.trim().is_empty();
            if button::button(ui, Kind::Outline, Size::Normal, &show_label, ready).clicked() {
                action = Some(Action::Show);
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let response = field_response(ui, &mut entity, &entity_hint, label.id);
                entity_changed |= response.changed();
                if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                    action = Some(Action::Show);
                }
            });
        });
    });
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
        None => {}
    }
}

fn triggers(ui: &mut Ui, model: &Model) -> Option<usize> {
    let texts = model.texts();
    let tab = model.sounds();
    match tab.list_status() {
        ListStatus::Empty => message(ui, texts.get(SoundsText::ListHint), palette::GRAY_2_TEXT),
        ListStatus::Loading => loading(ui, texts, SoundsText::ListLoading),
        ListStatus::NotAnEntity => {
            frames::warning_alert(ui, |ui| {
                ui.label(texts.get(SoundsText::NotAnEntity));
            });
        }
        ListStatus::Failed(reason) => {
            let text = texts.format(
                SoundsText::ListFailed,
                &[("reason", reason.to_owned().into())],
            );
            frames::danger_alert(ui, |ui| {
                ui.add(Label::new(text).selectable(true));
            });
        }
        ListStatus::ArchiveChanged => {
            frames::warning_alert(ui, |ui| {
                ui.label(texts.get(SoundsText::ListArchiveChanged));
            });
        }
        ListStatus::Ready(list) => return ready_list(ui, model, list),
    }
    None
}

fn ready_list(ui: &mut Ui, model: &Model, list: &TriggerList) -> Option<usize> {
    let texts = model.texts();
    let tab = model.sounds();
    let footer = count(texts, list);
    list_footer::show(ui, "sounds-footer", |ui| {
        if tab.is_listing() {
            indicator::spinner(ui, SPINNER_INLINE);
        }
        ui.add(
            Label::new(plain(footer, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT))
                .selectable(false),
        );
    });
    if list.rows.is_empty() {
        let text = match list.source {
            ListSource::Entity(_) => SoundsText::NoTriggers,
            ListSource::Search(_) => SoundsText::NoMatch,
        };
        message(ui, texts.get(text), palette::GRAY_2_TEXT);
        return None;
    }
    sounds_triggers::show(ui, texts, (list, tab.list_serial()), tab.selected())
}

fn count(texts: &Texts, list: &TriggerList) -> String {
    let number =
        |value: usize| format::number(u64::try_from(value).unwrap_or(u64::MAX), texts.language());
    let shown = u64::try_from(list.rows.len()).unwrap_or(u64::MAX);
    match &list.source {
        ListSource::Entity(entity) => texts.format(
            SoundsText::CountEntity,
            &[
                ("name", entity.class_name.clone().into()),
                ("count", FluentValue::from(shown)),
                ("number", number(list.rows.len()).into()),
                ("resolved", number(list.matched_count()).into()),
            ],
        ),
        ListSource::Search(_) if list.total > list.rows.len() => texts.format(
            SoundsText::CountSearchMore,
            &[
                ("number", number(list.rows.len()).into()),
                ("total", number(list.total).into()),
            ],
        ),
        ListSource::Search(_) => texts.format(
            SoundsText::CountSearch,
            &[
                ("count", FluentValue::from(shown)),
                ("number", number(list.rows.len()).into()),
            ],
        ),
    }
}

enum DetailAction {
    Select(usize),
    Audio(AudioAction),
}

fn detail(ui: &mut Ui, model: &Model) -> Option<DetailAction> {
    ui.set_min_height(ui.available_height());
    let texts = model.texts();
    match model.sounds().sounds_status() {
        SoundsListStatus::None => {
            message(ui, texts.get(SoundsText::SelectHint), palette::GRAY_2_TEXT);
        }
        SoundsListStatus::Loading => loading(ui, texts, SoundsText::VariantsLoading),
        SoundsListStatus::Undefined => {
            message(ui, texts.get(SoundsText::Undefined), palette::WARNING);
        }
        SoundsListStatus::Ready(list) => return sounds(ui, model, list),
    }
    None
}

fn sounds(ui: &mut Ui, model: &Model, list: &SoundList) -> Option<DetailAction> {
    let texts = model.texts();
    let tab = model.sounds();
    let total = u64::try_from(list.variants.len()).unwrap_or(u64::MAX);
    let heading = texts.format(
        SoundsText::VariantsCount,
        &[
            ("name", list.trigger.clone().into()),
            ("count", FluentValue::from(total)),
            ("number", format::number(total, texts.language()).into()),
        ],
    );
    ui.label(plain(heading, FS_SM, Weight::Bold).color(palette::GRAY_1));
    if !list.breaks.is_empty() {
        let breaks = u64::try_from(list.breaks.len()).unwrap_or(u64::MAX);
        let heading = texts.format(
            SoundsText::BreaksHeading,
            &[
                ("count", FluentValue::from(breaks)),
                ("number", format::number(breaks, texts.language()).into()),
            ],
        );
        record_export_outcome::list(ui, (&heading, "sound-breaks"), &list.breaks, None, false);
    }
    if list.variants.is_empty() {
        message(ui, texts.get(SoundsText::NoVariants), palette::GRAY_2_TEXT);
        return None;
    }
    let audio = selected(ui, model, list);
    let picked = sounds_variants::show(
        ui,
        texts,
        (list, tab.sounds_serial()),
        tab.selected_variant(),
    );
    picked
        .map(DetailAction::Select)
        .or_else(|| audio.map(DetailAction::Audio))
}

fn selected(ui: &mut Ui, model: &Model, list: &SoundList) -> Option<AudioAction> {
    let texts = model.texts();
    let tab = model.sounds();
    let Some(variant) = tab.selected_variant().and_then(|at| list.variants.get(at)) else {
        message(
            ui,
            texts.get(SoundsText::SelectVariant),
            palette::GRAY_2_TEXT,
        );
        return None;
    };
    let id = variant.wem_id.map(|id| id.to_string()).unwrap_or_default();
    let source = sounds_variants::source_name(texts, variant.kind);
    let detail = texts.format(
        SoundsText::Detail,
        &[
            ("path", variant.soundbank_path.clone().into()),
            ("id", id.clone().into()),
            ("source", source.into()),
        ],
    );
    ui.label(plain(detail, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT));
    let note = match variant.playable() {
        Playable::Yes => None,
        Playable::PrefetchOnly => Some(SoundsText::NotePrefetch),
        Playable::Plugin => Some(SoundsText::NotePlugin),
        Playable::Missing => Some(SoundsText::NoteMissing),
    };
    if let Some(note) = note {
        message(ui, texts.get(note), palette::WARNING);
        return None;
    }
    let audio = tab.audio()?;
    let name = texts.format(
        crate::AudioText::WemName,
        &[
            ("id", id.into()),
            ("bank", variant.soundbank.clone().into()),
        ],
    );
    let named = Named {
        path: &name,
        size: variant.size,
        embedded: variant.is_embedded(),
    };
    audio_preview::show(ui, texts, named, audio, model.audio_volume())
}

fn loading(ui: &mut Ui, texts: &Texts, text: SoundsText) {
    ui.add_space(SPINNER);
    ui.horizontal(|ui| {
        indicator::spinner(ui, SPINNER_INLINE);
        ui.add(
            Label::new(plain(texts.get(text), FS_SM, Weight::Regular).color(palette::GRAY_1))
                .selectable(false),
        );
    });
}

fn message(ui: &mut Ui, text: String, colour: Color32) {
    frames::preview_body().show(ui, |ui| {
        ui.add(Label::new(
            plain(text, FS_SM, Weight::Regular).color(colour),
        ));
    });
}
