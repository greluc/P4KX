use egui::{Align, Event, Label, Layout, Margin, Ui, vec2};
use fluent_bundle::FluentValue;

use super::localization_table::{self, Laid, Picked};
use super::search::{eyebrow, field};
use super::{empty, list_footer};
use crate::theme::desktop::{SPINNER, SPINNER_INLINE, TOOLBAR_PADDING};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, px};
use crate::widgets::indicator;
use crate::widgets::text::{hairline_below, plain};
use crate::{LocalizationStatus, LocalizationTable, LocalizationText, Model, Texts, format};

/// Shows the localisation tab: the search and copy actions, then the table of keys and texts
/// with the number of keys below it, or the state of reading the files.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    if model.browser().is_none() {
        let texts = model.texts();
        let title = texts.get(LocalizationText::EmptyTitle);
        let hint = texts.get(LocalizationText::EmptyHint);
        empty::notice(ui, &title, &hint);
        return;
    }
    if !model.localization().is_started() {
        let ctx = ui.ctx().clone();
        model.load_localization(move || ctx.request_repaint());
    }
    if model.localization().is_searching() {
        ui.ctx().request_repaint();
    }
    if matches!(model.localization().status(), LocalizationStatus::Ready(_)) {
        let toolbar = frames::panel()
            .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
            .show(ui, |ui| toolbar(ui, model));
        hairline_below(ui, toolbar.response.rect);
    }
    let picked = frames::panel()
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            body(ui, model)
        })
        .inner;
    if let Some(picked) = picked {
        model.localization_mut().select(picked.row, picked.language);
    }
    copy_shortcut(ui, model);
}

fn toolbar(ui: &mut Ui, model: &mut Model) {
    ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
    let texts = model.texts();
    let label = eyebrow(&texts.get(LocalizationText::Label));
    let hint = texts.get(LocalizationText::Hint);
    let copy_key = texts.get(LocalizationText::CopyKey);
    let copy_text = texts.get(LocalizationText::CopyText);
    let tab = model.localization();
    let mut text = tab.query().to_owned();
    let (key_enabled, text_enabled) = (
        tab.selected_key().is_some(),
        tab.selected().is_some_and(|(row, language)| {
            language.is_some_and(|language| tab_translated(model, row, language))
        }),
    );
    let mut changed = false;
    let mut copy = None;
    ui.horizontal(|ui| {
        let label = ui.label(label);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if button::button(ui, Kind::Ghost, Size::Normal, &copy_text, text_enabled).clicked() {
                copy = model.localization().selected_text();
            }
            if button::button(ui, Kind::Ghost, Size::Normal, &copy_key, key_enabled).clicked() {
                copy = model.localization().selected_key().map(str::to_owned);
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                changed = field(ui, &mut text, &hint, label.id);
            });
        });
    });
    if let Some(copy) = copy {
        ui.ctx().copy_text(copy);
    }
    if changed {
        let ctx = ui.ctx().clone();
        model
            .localization_mut()
            .set_query(&text, move || ctx.request_repaint());
    }
}

fn tab_translated(model: &Model, row: u32, language: crate::TextLanguage) -> bool {
    match model.localization().status() {
        LocalizationStatus::Ready(table) => table.is_translated(row, language),
        _ => false,
    }
}

fn body(ui: &mut Ui, model: &Model) -> Option<Picked> {
    let texts = model.texts();
    match model.localization().status() {
        LocalizationStatus::Idle | LocalizationStatus::Loading => loading(ui, texts),
        LocalizationStatus::NoTexts => message(
            ui,
            texts.get(LocalizationText::NoTexts),
            palette::GRAY_2_TEXT,
        ),
        LocalizationStatus::Failed(Some(reason)) => {
            let text = texts.format(
                LocalizationText::Failed,
                &[("reason", reason.to_owned().into())],
            );
            frames::danger_alert(ui, |ui| {
                ui.add(Label::new(text).selectable(true));
            });
        }
        LocalizationStatus::Failed(None) => {
            let text = texts.get(LocalizationText::Stopped);
            frames::danger_alert(ui, |ui| {
                ui.add(Label::new(text).selectable(true));
            });
        }
        LocalizationStatus::ArchiveChanged => {
            let text = texts.get(LocalizationText::ArchiveChanged);
            frames::warning_alert(ui, |ui| {
                ui.add(Label::new(text).selectable(true));
            });
        }
        LocalizationStatus::Ready(table) => return ready(ui, model, table),
    }
    None
}

fn ready(ui: &mut Ui, model: &Model, table: &LocalizationTable) -> Option<Picked> {
    let texts = model.texts();
    let tab = model.localization();
    if !table.notes().is_empty() {
        let heading = texts.get(LocalizationText::Notes);
        frames::warning_alert(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.add(Label::new(heading).selectable(true));
            for note in table.notes() {
                ui.add(Label::new(note.as_str()).selectable(true));
            }
        });
    }
    let count = count(texts, tab.shown_len(), table.len());
    let searching = tab
        .is_searching()
        .then(|| texts.get(LocalizationText::Searching));
    list_footer::show(ui, "localization-footer", |ui| {
        ui.add(
            Label::new(plain(count, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT))
                .selectable(false),
        );
        if let Some(searching) = searching {
            ui.add_space(TOOLBAR_PADDING);
            indicator::spinner(ui, SPINNER_INLINE);
            ui.add(
                Label::new(plain(searching, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT))
                    .selectable(false),
            );
        }
    });
    if tab.shown_len() == 0 {
        if !tab.is_searching() {
            message(
                ui,
                texts.get(LocalizationText::NoMatch),
                palette::GRAY_2_TEXT,
            );
        }
        return None;
    }
    let laid = Laid::get(ui, texts, tab, table);
    localization_table::show(ui, tab, table, &laid)
}

fn count(texts: &Texts, shown: usize, total: usize) -> String {
    let number =
        |value: usize| format::number(u64::try_from(value).unwrap_or(u64::MAX), texts.language());
    let count = FluentValue::from(u64::try_from(total).unwrap_or(u64::MAX));
    if shown == total {
        texts.format(
            LocalizationText::Keys,
            &[("count", count), ("number", number(total).into())],
        )
    } else {
        texts.format(
            LocalizationText::KeysShown,
            &[
                ("count", count),
                ("number", number(shown).into()),
                ("total", number(total).into()),
            ],
        )
    }
}

fn loading(ui: &mut Ui, texts: &Texts) {
    ui.add_space(SPINNER);
    ui.horizontal(|ui| {
        indicator::spinner(ui, SPINNER);
        ui.add(
            Label::new(
                plain(texts.get(LocalizationText::Loading), FS_SM, Weight::Regular)
                    .color(palette::GRAY_1),
            )
            .selectable(false),
        );
    });
}

fn message(ui: &mut Ui, text: String, colour: egui::Color32) {
    frames::preview_body().show(ui, |ui| {
        ui.add(Label::new(
            plain(text, FS_SM, Weight::Regular).color(colour),
        ));
    });
}

fn copy_shortcut(ui: &Ui, model: &Model) {
    let asked = ui.input(|input| {
        input
            .events
            .iter()
            .any(|event| matches!(event, Event::Copy))
    });
    if !asked || ui.ctx().egui_wants_keyboard_input() {
        return;
    }
    let tab = model.localization();
    if let Some(text) = tab
        .selected_text()
        .or_else(|| tab.selected_key().map(str::to_owned))
    {
        ui.ctx().copy_text(text);
    }
}
