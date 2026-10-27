// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Margin, Panel, Rect, Sense, Ui, pos2, vec2};

use super::{channels, regions, tree};
use crate::theme::desktop::{
    BUTTON_XS_HEIGHT, PAGE_CHEVRON, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH, SIDEBAR_WIDTH,
};
use crate::theme::tokens::SPACE_1;
use crate::theme::window_metrics::{SIDEBAR_COLLAPSE_WIDTH, SIDEBAR_RAIL_WIDTH};
use crate::widgets::button::ghost_icon;
use crate::widgets::frames::{self, px};
use crate::widgets::pagination::chevron;
use crate::widgets::tooltip;
use crate::{Model, Text};

/// Returns whether a window of `width` is too narrow for the sidebar to stand beside the work
/// surface unasked.
pub(super) fn collapses(width: f32) -> bool {
    width < SIDEBAR_COLLAPSE_WIDTH
}

/// Draws the sidebar: beside the work surface in a wide window; in a narrow one behind a rail
/// whose button shows and hides it, and opening at the minimum width of the sidebar.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let open_id = ui.make_persistent_id("sidebar-open");
    let narrow = collapses(ui.max_rect().width());
    let mut open = !narrow
        || ui
            .data(|data| data.get_temp::<bool>(open_id))
            .unwrap_or(false);
    if narrow {
        let rail = Panel::left("side-rail")
            .frame(frames::sidebar().inner_margin(Margin::same(px(SPACE_1))))
            .exact_size(SIDEBAR_RAIL_WIDTH - 2.0 * SPACE_1)
            .show(ui, |ui| {
                if toggle(ui, model, open) {
                    open = !open;
                    ui.data_mut(|data| data.insert_temp(open_id, open));
                }
            });
        regions::record(ui.ctx(), |regions| regions.rail = Some(rail.response.rect));
    }
    if !open {
        return;
    }
    let panel = if narrow {
        Panel::left("side-narrow").default_size(SIDEBAR_MIN_WIDTH)
    } else {
        Panel::left("side").default_size(SIDEBAR_WIDTH)
    };
    let side = panel
        .frame(frames::sidebar())
        .resizable(true)
        .size_range(SIDEBAR_MIN_WIDTH..=SIDEBAR_MAX_WIDTH)
        .show(ui, |ui| {
            channels::show(ui, model);
            tree::show(ui, model);
        });
    regions::record(ui.ctx(), |regions| {
        regions.sidebar = Some(side.response.rect);
    });
}

fn toggle(ui: &mut Ui, model: &Model, open: bool) -> bool {
    let label = model.texts().get(if open {
        Text::SidebarHide
    } else {
        Text::SidebarShow
    });
    let (rect, response) =
        ui.allocate_exact_size(vec2(BUTTON_XS_HEIGHT, BUTTON_XS_HEIGHT), Sense::click());
    ghost_icon(ui, &response, &label, |colour| {
        let icon = Rect::from_center_size(rect.center(), vec2(PAGE_CHEVRON, PAGE_CHEVRON));
        chevron(ui, pos2(icon.min.x, icon.min.y), open, colour);
    });
    tooltip::hover(response, |ui| {
        tooltip::line(ui, label.as_str());
    })
    .clicked()
}
