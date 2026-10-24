// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align, Color32, Label, Layout, Margin, Ui, vec2};
use fluent_bundle::FluentValue;

use super::search::{eyebrow, field};
use super::tags_table::{self, Action};
use super::{empty, list_footer};
use crate::theme::desktop::{SPINNER, SPINNER_INLINE, TOOLBAR_PADDING};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, px};
use crate::widgets::indicator;
use crate::widgets::text::{hairline_below, plain};
use crate::{Model, TagsStatus, TagsText, TagsView, Texts, format};

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
    if matches!(model.tags().status(), TagsStatus::Ready(_)) {
        let toolbar = frames::panel()
            .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
            .show(ui, |ui| toolbar(ui, model));
        hairline_below(ui, toolbar.response.rect);
    }
    let action = frames::panel()
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            body(ui, model)
        })
        .inner;
    match action {
        Some(Action::Select(position)) => model.tags_mut().select(position),
        Some(Action::Toggle(position)) => model.tags_mut().toggle(position),
        None => {}
    }
}

fn toolbar(ui: &mut Ui, model: &mut Model) {
    ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
    let texts = model.texts();
    let label = eyebrow(&texts.get(TagsText::Label));
    let hint = texts.get(TagsText::Hint);
    let copy_path = texts.get(TagsText::CopyPath);
    let copy_guid = texts.get(TagsText::CopyGuid);
    let tab = model.tags();
    let mut text = tab.query().to_owned();
    let path = tab.selected_path().map(str::to_owned);
    let guid = tab.selected_guid().map(|guid| guid.to_string());
    let mut changed = false;
    let mut copy = None;
    ui.horizontal(|ui| {
        let label = ui.label(label);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if button::button(ui, Kind::Ghost, Size::Normal, &copy_guid, guid.is_some()).clicked() {
                copy.clone_from(&guid);
            }
            if button::button(ui, Kind::Ghost, Size::Normal, &copy_path, path.is_some()).clicked() {
                copy.clone_from(&path);
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
        model.tags_mut().set_query(&text);
    }
}

fn body(ui: &mut Ui, model: &Model) -> Option<Action> {
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
        TagsStatus::Ready(view) => return ready(ui, texts, view),
    }
    None
}

fn ready(ui: &mut Ui, texts: &Texts, view: &TagsView) -> Option<Action> {
    if view.irregularities() > 0 {
        let irregular = u64::try_from(view.irregularities()).unwrap_or(u64::MAX);
        let number = format::number(irregular, texts.language());
        let text = texts.format(
            TagsText::Irregular,
            &[
                ("count", FluentValue::from(irregular)),
                ("number", number.into()),
            ],
        );
        frames::warning_alert(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.add(Label::new(text).selectable(true));
        });
    }
    let total = view.tree().len();
    let count = count(
        texts,
        if view.is_filtered() {
            view.shown_len()
        } else {
            total
        },
        total,
    );
    list_footer::show(ui, "tags-footer", |ui| {
        ui.add(
            Label::new(plain(count, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT))
                .selectable(false),
        );
    });
    if view.shown_len() == 0 {
        message(ui, texts.get(TagsText::NoMatch), palette::GRAY_2_TEXT);
        return None;
    }
    tags_table::show(ui, texts, view)
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
