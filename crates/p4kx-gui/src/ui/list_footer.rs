// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Frame, Margin, Panel, Stroke, Ui, vec2};

use crate::theme::desktop::{LIST_FOOTER_PADDING_X, LIST_FOOTER_PADDING_Y};
use crate::theme::palette;
use crate::widgets::frames::px;

/// Shows the `.list-panel__footer` at the bottom of `ui`: a hairline above `add_contents`, laid
/// out left to right.
pub(super) fn show(ui: &mut Ui, id: &'static str, add_contents: impl FnOnce(&mut Ui)) {
    Panel::bottom(id)
        .frame(Frame::NONE.inner_margin(Margin::symmetric(
            px(LIST_FOOTER_PADDING_X),
            px(LIST_FOOTER_PADDING_Y),
        )))
        .show_separator_line(false)
        .resizable(false)
        .show(ui, |ui| {
            let top = ui.max_rect().top() - LIST_FOOTER_PADDING_Y;
            ui.painter().hline(
                ui.max_rect()
                    .expand2(vec2(LIST_FOOTER_PADDING_X, 0.0))
                    .x_range(),
                top + 0.5,
                Stroke::new(1.0, palette::GRAY_3),
            );
            ui.horizontal(add_contents);
        });
}
