// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Margin, Panel, ScrollArea, Ui, vec2};

use super::{
    Requests, character_choices, character_chooser, character_report, character_toolbar,
    character_view, empty,
};
use crate::theme::assembly_metrics::{ASSEMBLY_PANEL_HEIGHT, ASSEMBLY_PANEL_MIN_HEIGHT};
use crate::theme::character_metrics::{CHARACTER_REPORT_HEIGHT, CHARACTER_STACK_WIDTH};
use crate::theme::desktop::{SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH, SIDEBAR_WIDTH, TOOLBAR_PADDING};
use crate::widgets::frames::{self, px};
use crate::widgets::text::hairline_below;
use crate::{CharacterText, EntityText, Model};

/// Shows the Characters tab: the choices of the Character and its 3D view with the report of what
/// could not be fitted, side by side, or in one scrolling column in a narrow window.
pub(super) fn show(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    if model.browser().is_none() {
        let texts = model.texts();
        let title = texts.get(EntityText::EmptyTitle);
        let hint = texts.get(CharacterText::EmptyHint);
        empty::notice(ui, &title, &hint);
        return;
    }
    if !model.characters().is_started() {
        let ctx = ui.ctx().clone();
        model.start_characters(move || ctx.request_repaint());
    }
    tick(ui, model);
    if ui.available_width() < CHARACTER_STACK_WIDTH {
        stacked(ui, model, requests);
    } else {
        let toolbar = frames::panel()
            .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
            .show(ui, |ui| character_toolbar::show(ui, model, requests, true));
        hairline_below(ui, toolbar.response.rect);
        side_by_side(ui, model, requests);
    }
    character_chooser::show(ui, model);
}

fn tick(ui: &Ui, model: &mut Model) {
    if let Some(playback) = model.characters_mut().playback_mut() {
        let seconds = ui.input(|input| input.stable_dt);
        if playback.tick(seconds) {
            ui.ctx().request_repaint();
        }
    }
}

fn side_by_side(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    Panel::left("characters-choices")
        .frame(frames::panel())
        .resizable(true)
        .default_size(SIDEBAR_WIDTH)
        .size_range(SIDEBAR_MIN_WIDTH..=SIDEBAR_MAX_WIDTH)
        .show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt("characters-choices-scroll")
                .auto_shrink(false)
                .show(ui, |ui| character_choices::show(ui, model, requests));
        });
    Panel::bottom("characters-report")
        .frame(frames::panel())
        .exact_size(CHARACTER_REPORT_HEIGHT)
        .show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt("characters-report-scroll")
                .auto_shrink(false)
                .show(ui, |ui| character_report::show(ui, model));
        });
    frames::panel().show(ui, |ui| {
        ui.set_min_height(ASSEMBLY_PANEL_MIN_HEIGHT);
        character_view::show(ui, model);
    });
}

fn stacked(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    ScrollArea::vertical()
        .id_salt("characters-stacked")
        .auto_shrink(false)
        .show(ui, |ui| {
            frames::panel()
                .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
                .show(ui, |ui| character_toolbar::show(ui, model, requests, false));
            character_choices::show(ui, model, requests);
            ui.allocate_ui(vec2(ui.available_width(), ASSEMBLY_PANEL_HEIGHT), |ui| {
                character_view::show(ui, model);
            });
            character_report::show(ui, model);
        });
}
