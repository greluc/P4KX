// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::{Arc, Mutex};

use egui::{Align, Color32, Label, Layout, Margin, Role, Sense, Ui, WidgetInfo, vec2};
use fluent_bundle::FluentValue;

use super::search::field;
use super::tags_cache::{Cache, Labels, Laid, memo};
use super::tags_table::{self, Action};
use super::{empty, list_footer};
use crate::theme::desktop::{SPINNER, SPINNER_INLINE, TOOLBAR_PADDING};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, px};
use crate::widgets::indicator;
use crate::widgets::text::{hairline_below, line, paint_left, plain};
use crate::{Model, TagsStatus, TagsText, TagsView, Texts, format};

/// What a copy button was asked to copy.
#[derive(Clone, Copy)]
enum Copied {
    Path,
    Guid,
}

/// Shows the Tags tab: the search and copy actions, then the tag tree with the number of tags
/// below it, or the state of reading the tags.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    if model.browser().is_none() {
        let texts = model.texts();
        let title = texts.get(TagsText::EmptyTitle);
        let hint = texts.get(TagsText::EmptyHint);
        empty::notice(ui, &title, &hint);
        return;
    }
    if !model.tags().is_started() {
        let ctx = ui.ctx().clone();
        model.load_tags(move || ctx.request_repaint());
    }
    let cache = Cache::get(ui, model.texts());
    if matches!(model.tags().status(), TagsStatus::Ready(_)) {
        let labels = Cache::lock(&cache).labels();
        let toolbar = frames::panel()
            .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
            .show(ui, |ui| toolbar(ui, model, &labels));
        hairline_below(ui, toolbar.response.rect);
    }
    let action = frames::panel()
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            body(ui, model, &cache)
        })
        .inner;
    match action {
        Some(Action::Select(position)) => model.tags_mut().select(position),
        Some(Action::Toggle(position)) => model.tags_mut().toggle(position),
        None => {}
    }
}

fn toolbar(ui: &mut Ui, model: &mut Model, labels: &Labels) {
    ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
    let tab = model.tags_mut();
    let has_path = tab.selected_path().is_some();
    let has_guid = tab.selected_guid().is_some();
    let mut copy = None;
    ui.horizontal(|ui| {
        let label = label(ui, &labels.eyebrow);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if button::button(ui, Kind::Ghost, Size::Normal, &labels.copy_guid, has_guid).clicked()
            {
                copy = Some(Copied::Guid);
            }
            if button::button(ui, Kind::Ghost, Size::Normal, &labels.copy_path, has_path).clicked()
            {
                copy = Some(Copied::Path);
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                tab.edit_query(|text| field(ui, text, &labels.hint, label));
            });
        });
    });
    let copied = match copy {
        Some(Copied::Path) => tab.selected_path().map(str::to_owned),
        Some(Copied::Guid) => tab.selected_guid().map(|guid| guid.to_string()),
        None => None,
    };
    if let Some(copied) = copied {
        ui.ctx().copy_text(copied);
    }
}

/// Shows the laid-out label text and returns the id that names the widgets it labels.
fn label(ui: &mut Ui, laid: &Laid) -> egui::Id {
    let (rect, response) = ui.allocate_exact_size(laid.galley.size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &*laid.text));
    paint_left(ui, rect, laid.galley.clone(), palette::GRAY_2_TEXT);
    response.id
}

fn body(ui: &mut Ui, model: &Model, cache: &Mutex<Cache>) -> Option<Action> {
    let texts = model.texts();
    match model.tags().status() {
        TagsStatus::Idle | TagsStatus::Loading => loading(ui, texts),
        TagsStatus::NoTags => message(ui, texts.get(TagsText::NoTags), palette::GRAY_2_TEXT),
        TagsStatus::Failed(Some(reason)) => {
            let text = texts.format(TagsText::Failed, &[("reason", reason.to_owned().into())]);
            frames::danger_alert(ui, |ui| {
                ui.add(Label::new(text).selectable(true));
            });
        }
        TagsStatus::Failed(None) => {
            let text = texts.get(TagsText::Stopped);
            frames::danger_alert(ui, |ui| {
                ui.add(Label::new(text).selectable(true));
            });
        }
        TagsStatus::ArchiveChanged => {
            let text = texts.get(TagsText::ArchiveChanged);
            frames::warning_alert(ui, |ui| {
                ui.add(Label::new(text).selectable(true));
            });
        }
        TagsStatus::Ready(view) => return ready(ui, texts, view, cache),
    }
    None
}

fn ready(ui: &mut Ui, texts: &Texts, view: &TagsView, cache: &Mutex<Cache>) -> Option<Action> {
    let irregularities = view.irregularities();
    if irregularities > 0 {
        let text = irregular_text(cache, texts, irregularities);
        frames::warning_alert(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.add(Label::new(&*text).selectable(true));
        });
    }
    let total = view.tree().len();
    let shown = if view.is_filtered() {
        view.shown_len()
    } else {
        total
    };
    let counted = counted(ui, cache, texts, (shown, total));
    list_footer::show(ui, "tags-footer", |ui| {
        let (rect, response) = ui.allocate_exact_size(counted.galley.size(), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &*counted.text));
        paint_left(ui, rect, counted.galley.clone(), palette::GRAY_2_TEXT);
    });
    if view.shown_len() == 0 {
        message(ui, texts.get(TagsText::NoMatch), palette::GRAY_2_TEXT);
        return None;
    }
    tags_table::show(ui, texts, view, cache)
}

/// Returns the note about the tree differing from the `TagDatabase`, made once per count.
fn irregular_text(cache: &Mutex<Cache>, texts: &Texts, irregularities: usize) -> Arc<str> {
    let mut cache = Cache::lock(cache);
    memo(&mut cache.irregular, irregularities, || {
        irregular(texts, irregularities).into()
    })
    .clone()
}

/// Returns the footer text with the number of tags, laid out once per count.
fn counted(ui: &Ui, cache: &Mutex<Cache>, texts: &Texts, (shown, total): (usize, usize)) -> Laid {
    let mut cache = Cache::lock(cache);
    memo(&mut cache.count, (shown, total), || {
        let text = count(texts, shown, total);
        Laid {
            galley: line(
                ui,
                plain(&text, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT),
                f32::INFINITY,
            ),
            text: text.into(),
        }
    })
    .clone()
}

fn irregular(texts: &Texts, irregularities: usize) -> String {
    let irregular = u64::try_from(irregularities).unwrap_or(u64::MAX);
    let number = format::number(irregular, texts.language());
    texts.format(
        TagsText::Irregular,
        &[
            ("count", FluentValue::from(irregular)),
            ("number", number.into()),
        ],
    )
}

fn count(texts: &Texts, shown: usize, total: usize) -> String {
    let number =
        |value: usize| format::number(u64::try_from(value).unwrap_or(u64::MAX), texts.language());
    let count = FluentValue::from(u64::try_from(total).unwrap_or(u64::MAX));
    if shown == total {
        texts.format(
            TagsText::Count,
            &[("count", count), ("number", number(total).into())],
        )
    } else {
        texts.format(
            TagsText::CountShown,
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
        indicator::spinner(ui, SPINNER_INLINE);
        ui.add(
            Label::new(
                plain(texts.get(TagsText::Loading), FS_SM, Weight::Regular).color(palette::GRAY_1),
            )
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
