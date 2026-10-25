// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Color32, Margin, Painter, Panel, Pos2, Rect, Role, Sense, Stroke, Ui, WidgetInfo};
use egui::{pos2, vec2};

use super::{channels, tree};
use crate::theme::desktop::{
    BUTTON_XS_HEIGHT, CHEVRON_STROKE, FOCUS_OFFSET, SECTION_CHEVRON, SIDEBAR_MAX_WIDTH,
    SIDEBAR_MIN_WIDTH, SIDEBAR_WIDTH,
};
use crate::theme::palette;
use crate::theme::tokens::SPACE_1;
use crate::theme::window_metrics::{SIDEBAR_COLLAPSE_WIDTH, SIDEBAR_RAIL_WIDTH};
use crate::widgets::button::{Kind, Phase, look, paint};
use crate::widgets::focus::focus_ring;
use crate::widgets::frames::{self, px};
use crate::widgets::tooltip;
use crate::{Model, Text};

/// Returns whether a window of `width` is too narrow for the sidebar to stand beside the work
/// surface unasked.
pub(super) fn collapses(width: f32) -> bool {
    width < SIDEBAR_COLLAPSE_WIDTH
}

/// Draws the sidebar: beside the work surface in a wide window; in a narrow one behind a rail
/// whose button shows and hides it.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let open_id = ui.make_persistent_id("sidebar-open");
    let narrow = collapses(ui.max_rect().width());
    let mut open = !narrow
        || ui
            .data(|data| data.get_temp::<bool>(open_id))
            .unwrap_or(false);
    if narrow {
        Panel::left("side-rail")
            .frame(frames::sidebar().inner_margin(Margin::same(px(SPACE_1))))
            .exact_size(SIDEBAR_RAIL_WIDTH)
            .show(ui, |ui| {
                if toggle(ui, model, open) {
                    open = !open;
                    ui.data_mut(|data| data.insert_temp(open_id, open));
                }
            });
    }
    if !open {
        return;
    }
    Panel::left("side")
        .frame(frames::sidebar())
        .resizable(true)
        .default_size(if narrow {
            SIDEBAR_MIN_WIDTH
        } else {
            SIDEBAR_WIDTH
        })
        .size_range(SIDEBAR_MIN_WIDTH..=SIDEBAR_MAX_WIDTH)
        .show(ui, |ui| {
            channels::show(ui, model);
            tree::show(ui, model);
        });
}

fn toggle(ui: &mut Ui, model: &Model, open: bool) -> bool {
    let label = model.texts().get(if open {
        Text::SidebarHide
    } else {
        Text::SidebarShow
    });
    let rect = Rect::from_min_size(ui.max_rect().min, vec2(BUTTON_XS_HEIGHT, BUTTON_XS_HEIGHT));
    ui.allocate_rect(rect, Sense::hover());
    let response = ui.interact(
        rect,
        ui.make_persistent_id("sidebar-toggle"),
        Sense::click(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &label));
    let phase = if response.is_pointer_button_down_on() {
        Phase::Pressed
    } else if response.hovered() {
        Phase::Hover
    } else {
        Phase::Rest
    };
    let colours = look(Kind::Ghost, phase);
    paint(ui, rect, colours);
    arrow(ui.painter(), rect.center(), open, colours.text);
    if response.has_focus() {
        focus_ring(ui, rect, palette::PRIMARY, FOCUS_OFFSET);
    }
    tooltip::hover(response, |ui| {
        tooltip::line(ui, label.as_str());
    })
    .clicked()
}

fn arrow(painter: &Painter, centre: Pos2, left: bool, colour: Color32) {
    let (quarter, half) = (SECTION_CHEVRON / 4.0, SECTION_CHEVRON / 2.0);
    let stroke = Stroke::new(CHEVRON_STROKE.min(quarter), colour);
    let side: f32 = if left { -1.0 } else { 1.0 };
    let tip = pos2(side.mul_add(quarter, centre.x), centre.y);
    let base = side.mul_add(-quarter, centre.x);
    painter.line_segment([pos2(base, centre.y - half), tip], stroke);
    painter.line_segment([tip, pos2(base, centre.y + half)], stroke);
}
