// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align, Label, Layout, Margin, Panel, Ui, vec2};
use fluent_bundle::FluentValue;

use super::entity_list;
use super::entity_loadout;
use super::entity_view;
use super::search::{eyebrow, field};
use super::{empty, entity_replace, list_footer};
use crate::theme::assembly_metrics::{
    ASSEMBLY_PANEL_HEIGHT, ASSEMBLY_PANEL_MIN_HEIGHT, ASSEMBLY_PANEL_SHARE,
};
use crate::theme::desktop::{
    SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH, SPINNER, SPINNER_INLINE, TOOLBAR_PADDING,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, px};
use crate::widgets::indicator;
use crate::widgets::tab::{self, SegmentLabel};
use crate::widgets::text::{hairline_below, plain};
use crate::{EntitiesStatus, EntityFilter, EntityList, EntityText, Model, Texts, format};

/// Shows the Entities tab: the search and the export action, the kinds to list, then the picker
/// beside the Loadout of the selected Entity, or the state of listing the Entities.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    if model.browser().is_none() {
        let texts = model.texts();
        let title = texts.get(EntityText::EmptyTitle);
        let hint = texts.get(EntityText::EmptyHint);
        empty::notice(ui, &title, &hint);
        return;
    }
    if !model.entities().is_started() {
        let ctx = ui.ctx().clone();
        model.load_entities(move || ctx.request_repaint());
    }
    if matches!(model.entities().status(), EntitiesStatus::Ready(_)) {
        let toolbar = frames::panel()
            .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
            .show(ui, |ui| toolbar(ui, model));
        hairline_below(ui, toolbar.response.rect);
    }
    frames::panel().show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        body(ui, model);
    });
    entity_replace::show(ui, model);
}

fn toolbar(ui: &mut Ui, model: &mut Model) {
    ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
    let texts = model.texts();
    let label = eyebrow(&texts.get(EntityText::Label));
    let hint = texts.get(EntityText::Hint);
    let export = texts.get(EntityText::Export);
    let available = model.assembly_export_available();
    let mut text = model.entities().query().to_owned();
    let (mut changed, mut open) = (false, false);
    ui.horizontal(|ui| {
        let label = ui.label(label);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            open = button::button(ui, Kind::Cta, Size::Normal, &export, available).clicked();
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                changed = field(ui, &mut text, &hint, label.id);
            });
        });
    });
    let chosen = filters(ui, model);
    if changed {
        model.entities_mut().set_query(&text);
    }
    if let Some(filter) = chosen {
        model.entities_mut().set_filter(filter);
    }
    if open {
        model.begin_assembly_export();
    }
}

fn filters(ui: &mut Ui, model: &Model) -> Option<EntityFilter> {
    let EntitiesStatus::Ready(list) = model.entities().status() else {
        return None;
    };
    let texts = model.texts();
    let labels: Vec<SegmentLabel> = EntityFilter::ALL
        .iter()
        .map(|filter| {
            let count = number(texts, list.total(*filter));
            SegmentLabel::new(ui, &texts.get(filter_text(*filter)), Some(&count))
        })
        .collect();
    let active = EntityFilter::ALL
        .iter()
        .position(|filter| *filter == model.entities().filter());
    ui.horizontal(|ui| tab::counted_segments(ui, &labels, active))
        .inner
        .and_then(|at| EntityFilter::ALL.get(at).copied())
}

const fn filter_text(filter: EntityFilter) -> EntityText {
    match filter {
        EntityFilter::All => EntityText::FilterAll,
        EntityFilter::Ships => EntityText::FilterShips,
        EntityFilter::Vehicles => EntityText::FilterVehicles,
        EntityFilter::Weapons => EntityText::FilterWeapons,
    }
}

fn body(ui: &mut Ui, model: &mut Model) {
    let texts = model.texts();
    match model.entities().status() {
        EntitiesStatus::Idle | EntitiesStatus::Loading => loading(ui, texts),
        EntitiesStatus::NoEntities => message(ui, texts.get(EntityText::NoEntities)),
        EntitiesStatus::Failed(Some(reason)) => {
            let text = texts.format(EntityText::Failed, &[("reason", reason.to_owned().into())]);
            frames::danger_alert(ui, |ui| {
                ui.add(Label::new(text).selectable(true));
            });
        }
        EntitiesStatus::Failed(None) => {
            let text = texts.get(EntityText::Stopped);
            frames::danger_alert(ui, |ui| {
                ui.add(Label::new(text).selectable(true));
            });
        }
        EntitiesStatus::ArchiveChanged => {
            let text = texts.get(EntityText::ArchiveChanged);
            frames::warning_alert(ui, |ui| {
                ui.add(Label::new(text).selectable(true));
            });
        }
        EntitiesStatus::Ready(_) => ready(ui, model),
    }
}

fn ready(ui: &mut Ui, model: &mut Model) {
    let picked = Panel::left("entities-list")
        .frame(frames::panel())
        .resizable(true)
        .default_size(SIDEBAR_MAX_WIDTH.min(ui.available_width() / 2.0))
        .size_range(SIDEBAR_MIN_WIDTH..=SIDEBAR_MAX_WIDTH)
        .show(ui, |ui| {
            let EntitiesStatus::Ready(list) = model.entities().status() else {
                return None;
            };
            let shown = (model.entities().filter(), model.entities().query());
            footer(ui, model.texts(), list, shown);
            if list.shown_len() == 0 {
                message(ui, model.texts().get(EntityText::NoMatch));
                return None;
            }
            entity_list::show(ui, list, model.entities().selected())
        })
        .inner;
    if let Some(position) = picked {
        let ctx = ui.ctx().clone();
        model.select_entity(position, move || ctx.request_repaint());
    }
    if model.entities().loadout().is_some() {
        let least = ASSEMBLY_PANEL_MIN_HEIGHT;
        let most = (ui.available_height() - entity_view::TREE_MIN_HEIGHT).max(least);
        let support = model.viewer_support().clone();
        Panel::top("entities-view")
            .frame(frames::panel())
            .resizable(true)
            .default_size(
                (ui.available_height() * ASSEMBLY_PANEL_SHARE)
                    .max(ASSEMBLY_PANEL_HEIGHT)
                    .clamp(least, most),
            )
            .size_range(least..=most)
            .show(ui, |ui| {
                let (tab, texts) = model.entities_with_texts();
                entity_view::show(ui, tab, texts, &support);
            });
    }
    frames::panel().show(ui, |ui| {
        frames::preview_body().show(ui, |ui| entity_loadout::show(ui, model));
    });
}

fn footer(ui: &mut Ui, texts: &Texts, list: &EntityList, (filter, query): (EntityFilter, &str)) {
    let total = list.total(filter);
    let shown = list.shown_len();
    let value = |count: usize| FluentValue::from(u64::try_from(count).unwrap_or(u64::MAX));
    let numbered = |count: usize| number(texts, count);
    let text = if shown == total && query.is_empty() {
        texts.format(
            EntityText::Count,
            &[("count", value(total)), ("number", numbered(total).into())],
        )
    } else {
        texts.format(
            EntityText::CountShown,
            &[
                ("count", value(total)),
                ("number", numbered(shown).into()),
                ("total", numbered(total).into()),
            ],
        )
    };
    list_footer::show(ui, "entities-footer", |ui| {
        ui.add(
            Label::new(plain(text, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT))
                .selectable(false),
        );
    });
}

fn number(texts: &Texts, value: usize) -> String {
    format::number(u64::try_from(value).unwrap_or(u64::MAX), texts.language())
}

fn loading(ui: &mut Ui, texts: &Texts) {
    ui.add_space(SPINNER);
    ui.horizontal(|ui| {
        indicator::spinner(ui, SPINNER_INLINE);
        ui.add(
            Label::new(
                plain(texts.get(EntityText::Loading), FS_SM, Weight::Regular)
                    .color(palette::GRAY_1),
            )
            .selectable(false),
        );
    });
}

fn message(ui: &mut Ui, text: String) {
    frames::preview_body().show(ui, |ui| {
        ui.add(Label::new(
            plain(text, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT),
        ));
    });
}
