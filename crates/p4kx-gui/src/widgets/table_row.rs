// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::layers::ShapeIdx;
use egui::{CornerRadius, Response, Shape, Stroke, Ui, pos2, vec2};

use super::rows::RowState;
use crate::theme::desktop::{ACTIVE_BAR, ZEBRA};
use crate::theme::palette;

/// The places under the cells of a table row where its background is painted once the row has
/// its rectangle: `egui_extras` clips the first cell to its column, so the cell cannot paint the
/// whole row itself.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RowSlots {
    fill: ShapeIdx,
    bar: ShapeIdx,
    divider: ShapeIdx,
}

impl RowSlots {
    /// Reserves the places in the first cell `ui` of a row.
    pub(crate) fn reserve(ui: &Ui) -> Self {
        let painter = ui.painter();
        Self {
            fill: painter.add(Shape::Noop),
            bar: painter.add(Shape::Noop),
            divider: painter.add(Shape::Noop),
        }
    }

    /// Paints the fill of `state` (the zebra stripe when `striped` and at rest), the orange bar of
    /// the active row and the divider across the whole `row`, behind its cells.
    pub(crate) fn paint(self, row: &Response, state: RowState, striped: bool) {
        let painter = row
            .ctx
            .layer_painter(row.layer_id)
            .with_clip_rect(row.interact_rect);
        let rect = row.rect;
        let (fill, _) = state.colours();
        let fill = if striped && state == RowState::Rest {
            ZEBRA
        } else {
            fill
        };
        painter.set(
            self.fill,
            Shape::rect_filled(rect, CornerRadius::ZERO, fill),
        );
        if state == RowState::Active {
            let bar = egui::Rect::from_min_size(rect.min, vec2(ACTIVE_BAR, rect.height()));
            painter.set(
                self.bar,
                Shape::rect_filled(bar, CornerRadius::ZERO, palette::PRIMARY),
            );
        }
        let y = rect.bottom() - 0.5;
        painter.set(
            self.divider,
            Shape::line_segment(
                [pos2(rect.left(), y), pos2(rect.right(), y)],
                Stroke::new(1.0, palette::GRAY_3),
            ),
        );
    }
}
