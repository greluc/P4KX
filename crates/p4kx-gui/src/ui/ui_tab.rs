// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Label, Margin, Panel, Ui};

use super::ui_tab_list::{self, Picked};
use super::ui_tab_preview;
use super::ui_tab_toolbar;
use super::{Requests, empty};
use crate::theme::desktop::{
    SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH, SIDEBAR_WIDTH, SPINNER, SPINNER_INLINE, TOOLBAR_PADDING,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, Weight};
use crate::widgets::frames::{self, px};
use crate::widgets::indicator;
use crate::widgets::text::{hairline_below, plain};
use crate::{EntitiesStatus, EntityText, Model, Texts, UiFilesStatus, UiMode, UiText};

/// Shows the UI tab: the source, the render settings and the export, then the list to pick from
/// beside the picture and what it is made of and what it leaves out.
pub(super) fn show(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    if model.browser().is_none() {
        let texts = model.texts();
        let title = texts.get(UiText::EmptyTitle);
        let hint = texts.get(UiText::EmptyHint);
        empty::notice(ui, &title, &hint);
        return;
    }
    start(ui, model);
    let toolbar = frames::panel()
        .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui_tab_toolbar::show(ui, model, requests);
        });
    hairline_below(ui, toolbar.response.rect);
    frames::panel().show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        match model.ui().mode() {
            UiMode::Files => files(ui, model),
            UiMode::Ships => ships(ui, model),
        }
    });
}

fn start(ui: &Ui, model: &mut Model) {
    let wake = || {
        let ctx = ui.ctx().clone();
        move || ctx.request_repaint()
    };
    match model.ui().mode() {
        UiMode::Files => {
            model.load_ui_files(wake());
        }
        UiMode::Ships => {
            if !model.entities().is_started() {
                model.load_entities(wake());
            }
        }
    }
    model.sync_ui(wake());
}

fn files(ui: &mut Ui, model: &mut Model) {
    let texts = model.texts();
    match model.ui().files_status() {
        UiFilesStatus::Idle | UiFilesStatus::Loading => loading(ui, texts, UiText::FilesLoading),
        UiFilesStatus::NoFiles => message(ui, texts.get(UiText::FilesNone)),
        UiFilesStatus::Failed(Some(reason)) => danger(
            ui,
            texts.format(UiText::FilesFailed, &[("reason", reason.to_owned().into())]),
        ),
        UiFilesStatus::Failed(None) => danger(ui, texts.get(UiText::FilesStopped)),
        UiFilesStatus::ArchiveChanged => warning(ui, texts.get(UiText::FilesArchiveChanged)),
        UiFilesStatus::Ready(_) => {
            let picked = list_panel(ui, "ui-files", |ui| ui_tab_list::files(ui, model));
            if let Some(Picked::File(position)) = picked {
                model.ui_mut().select_file(position);
            }
            ui_tab_preview::show(ui, model);
        }
    }
}

fn ships(ui: &mut Ui, model: &mut Model) {
    let texts = model.texts();
    match model.entities().status() {
        EntitiesStatus::Idle | EntitiesStatus::Loading => loading(ui, texts, EntityText::Loading),
        EntitiesStatus::NoEntities => message(ui, texts.get(UiText::ShipsNone)),
        EntitiesStatus::Failed(Some(reason)) => danger(
            ui,
            texts.format(EntityText::Failed, &[("reason", reason.to_owned().into())]),
        ),
        EntitiesStatus::Failed(None) => danger(ui, texts.get(EntityText::Stopped)),
        EntitiesStatus::ArchiveChanged => warning(ui, texts.get(EntityText::ArchiveChanged)),
        EntitiesStatus::Ready(_) if model.ui().ships().is_none() => {
            loading(ui, texts, EntityText::Loading);
        }
        EntitiesStatus::Ready(_) => {
            let picked = list_panel(ui, "ui-ships", |ui| ui_tab_list::ships(ui, model));
            if let Some(Picked::Ship(position)) = picked {
                let ctx = ui.ctx().clone();
                model.select_ui_ship(position, move || ctx.request_repaint());
            }
            if model.ui().ship().is_some() {
                let screen = Panel::left("ui-screens")
                    .frame(frames::panel())
                    .resizable(true)
                    .default_size(SIDEBAR_WIDTH.min(ui.available_width() / 3.0))
                    .size_range(
                        SIDEBAR_MIN_WIDTH.min(ui.available_width() / 4.0)..=SIDEBAR_MAX_WIDTH,
                    )
                    .show(ui, |ui| ui_tab_list::screens(ui, model))
                    .inner;
                if let Some(Picked::Screen(position)) = screen {
                    model.ui_mut().pick_screen(position);
                }
            }
            ui_tab_preview::show(ui, model);
        }
    }
}

fn list_panel(
    ui: &mut Ui,
    id: &'static str,
    add: impl FnOnce(&mut Ui) -> Option<Picked>,
) -> Option<Picked> {
    Panel::left(id)
        .frame(frames::panel())
        .resizable(true)
        .default_size(SIDEBAR_WIDTH.min(ui.available_width() / 2.0))
        .size_range(SIDEBAR_MIN_WIDTH.min(ui.available_width() / 3.0)..=SIDEBAR_MAX_WIDTH)
        .show(ui, add)
        .inner
}

/// Shows a spinner with the text `text` while something is read.
pub(super) fn loading(ui: &mut Ui, texts: &Texts, text: impl Into<crate::Text>) {
    ui.add_space(SPINNER);
    ui.horizontal(|ui| {
        indicator::spinner(ui, SPINNER_INLINE);
        ui.add(
            Label::new(plain(texts.get(text), FS_SM, Weight::Regular).color(palette::GRAY_1))
                .selectable(false),
        );
    });
}

/// Shows `text` as a quiet note.
pub(super) fn message(ui: &mut Ui, text: String) {
    frames::preview_body().show(ui, |ui| {
        ui.add(Label::new(
            plain(text, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT),
        ));
    });
}

/// Shows `text` as a failure.
pub(super) fn danger(ui: &mut Ui, text: String) {
    frames::danger_alert(ui, |ui| {
        ui.add(Label::new(text).selectable(true));
    });
}

/// Shows `text` as a warning.
pub(super) fn warning(ui: &mut Ui, text: String) {
    frames::warning_alert(ui, |ui| {
        ui.add(Label::new(text).selectable(true));
    });
}
