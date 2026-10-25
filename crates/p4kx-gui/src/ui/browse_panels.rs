// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Margin, Panel, Ui};

use super::{RESIZE_GAP, preview, record_search, regions, search, table};
use crate::theme::desktop::{PREVIEW_HEIGHT, PREVIEW_MIN_HEIGHT};
use crate::theme::model_metrics::PREVIEW_MODEL_HEIGHT;
use crate::theme::sound_metrics::PREVIEW_SOUNDBANK_HEIGHT;
use crate::theme::window_metrics::BROWSE_LIST_MIN_HEIGHT;
use crate::widgets::frames;
use crate::{Model, PreviewTab};

/// Shows the Browse tab of an open Archive: the preview below, capped so that the Entry list
/// above keeps [`BROWSE_LIST_MIN_HEIGHT`], and the list with its toolbar.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let soundbank = model.preview_tab() == PreviewTab::Audio
        && model
            .browser()
            .is_some_and(|browser| browser.soundbank().is_some());
    let (salt, height) = if soundbank {
        ("preview-soundbank", PREVIEW_SOUNDBANK_HEIGHT)
    } else if model.preview_tab() == PreviewTab::Model {
        ("preview-model", PREVIEW_MODEL_HEIGHT)
    } else {
        ("preview", PREVIEW_HEIGHT)
    };
    let room = ui.available_height() - f32::from(RESIZE_GAP) - BROWSE_LIST_MIN_HEIGHT;
    let preview = Panel::bottom(salt)
        .frame(frames::panel().outer_margin(Margin {
            top: RESIZE_GAP,
            ..Margin::ZERO
        }))
        .resizable(true)
        .default_size(height)
        .size_range(PREVIEW_MIN_HEIGHT..=room.max(PREVIEW_MIN_HEIGHT))
        .show(ui, |ui| {
            preview::show(ui, model);
            ui.min_rect()
        });
    let preview_rect = preview.response.rect.union(preview.inner);
    let list = frames::panel().show(ui, |ui| {
        if model.browser().is_some_and(crate::Browser::shows_records) {
            record_search::show(ui, model);
        } else {
            search::show(ui, model);
            table::show(ui, model);
        }
    });
    regions::record(ui.ctx(), |regions| {
        regions.preview = Some(preview_rect);
        regions.list = Some(list.response.rect);
    });
}
