// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align, Frame, Id, Layout, Margin, Rect, TextEdit, Ui, pos2, vec2};

use super::extraction;
use crate::theme::desktop::{
    ICON, SEARCH_ICON_INSET, SEARCH_PADDING_LEFT, SEARCH_WIDTH, TOOLBAR_PADDING,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_2XS, FS_BODY, TOUCH_TARGET, TRACKING_OVERLINE, Weight, font};
use crate::widgets::frames::px;
use crate::widgets::text::{caps, hairline_below, plain};
use crate::widgets::{field, indicator, tab};
use crate::{Model, Text};

/// The Filters the quick-filter chips apply.
const QUICK_FILTERS: [&str; 5] = ["*.xml", "*.dds", "*.dcb", "Game2.dcb", "aurora"];

/// Shows the toolbar of the list panel: the search field, the Extraction actions and the quick
/// filters.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let Some(browser) = model.browser() else {
        return;
    };
    let mut text = browser.search_text().to_owned();
    let texts = model.texts();
    let label = eyebrow(&texts.get(Text::SearchLabel));
    let hint = texts.get(Text::SearchHint);
    let filters = eyebrow(&texts.get(Text::QuickFilters));
    let mut changed = false;
    let toolbar = Frame::NONE
        .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(TOOLBAR_PADDING, TOOLBAR_PADDING);
            let mut stacked = false;
            ui.horizontal(|ui| {
                let label = ui.label(label);
                stacked = !extraction::fit_beside(ui, model, SEARCH_WIDTH);
                if stacked {
                    changed |= field(ui, &mut text, &hint, label.id);
                    return;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    extraction::actions(ui, model);
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        changed |= field(ui, &mut text, &hint, label.id);
                    });
                });
            });
            if stacked {
                extraction::stacked_actions(ui, model);
            }
            ui.horizontal_wrapped(|ui| {
                ui.label(filters);
                let active = QUICK_FILTERS.iter().position(|chip| text == *chip);
                if let Some(index) = tab::segments(ui, &QUICK_FILTERS, active) {
                    text = match (active == Some(index), QUICK_FILTERS.get(index)) {
                        (false, Some(chip)) => (*chip).to_owned(),
                        _ => String::new(),
                    };
                    changed = true;
                }
            });
        });
    hairline_below(ui, toolbar.response.rect);
    if !changed {
        return;
    }
    let ctx = ui.ctx().clone();
    if let Some(browser) = model.browser_mut() {
        browser.search(&text, move || ctx.request_repaint());
    }
}

pub(super) fn eyebrow(text: &str) -> egui::RichText {
    caps(text, FS_2XS, Weight::Bold, TRACKING_OVERLINE).color(palette::GRAY_2_TEXT)
}

pub(super) fn field(ui: &mut Ui, text: &mut String, hint: &str, label: Id) -> bool {
    field_response(ui, text, hint, label).changed()
}

/// Shows the search field like [`field`] and returns its response.
pub(super) fn field_response(
    ui: &mut Ui,
    text: &mut String,
    hint: &str,
    label: Id,
) -> egui::Response {
    let bloom = field::bloom_slot(ui);
    let frame = field::frame_slot(ui);
    let edit = TextEdit::singleline(text)
        .hint_text(plain(hint, FS_BODY, Weight::Regular).italics())
        .font(font(FS_BODY, Weight::Regular))
        .frame(Frame::NONE.inner_margin(Margin {
            left: px(SEARCH_PADDING_LEFT),
            right: px(TOOLBAR_PADDING),
            top: 0,
            bottom: 0,
        }))
        .min_size(vec2(SEARCH_WIDTH.min(ui.available_width()), TOUCH_TARGET))
        .desired_width(ui.available_width())
        .align(egui::Align2::LEFT_CENTER);
    let response = ui.add(edit).labelled_by(label);
    field::paint_frame(ui, frame, response.rect, response.has_focus());
    if response.has_focus() {
        field::paint_bloom(ui, bloom, response.rect);
    }
    let icon = Rect::from_min_size(
        pos2(
            response.rect.left() + SEARCH_ICON_INSET,
            response.rect.center().y - ICON / 2.0,
        ),
        vec2(ICON, ICON),
    );
    indicator::search_icon(ui, icon, palette::GRAY_2_TEXT);
    response
}
