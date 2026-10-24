// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Rect, Role, ScrollArea, Sense, Ui, WidgetInfo, pos2, vec2};
use p4kx_extraction::Guid;

use crate::EntityList;
use crate::theme::desktop::{CHANNEL_GAP, ROW_FONT, ROW_HEIGHT, SCROLLBAR_THIN, TREE_PADDING_LEFT};
use crate::theme::palette;
use crate::theme::tokens::SPACE_2;
use crate::widgets::rows::{RowState, focus_inside, paint_row, row_weight};
use crate::widgets::text::{line, paint_left, plain};
use crate::widgets::tooltip;

/// Shows the Entities of the picker as rows with the class name and the record path; returns the
/// position of the row the user clicked.
pub(super) fn show(ui: &mut Ui, list: &EntityList, selected: Option<Guid>) -> Option<usize> {
    let mut clicked = None;
    ui.spacing_mut().scroll.bar_width = SCROLLBAR_THIN;
    ui.spacing_mut().item_spacing.y = 0.0;
    ScrollArea::vertical()
        .id_salt("entities-rows")
        .auto_shrink(false)
        .show_rows(ui, ROW_HEIGHT, list.shown_len(), |ui, visible| {
            for position in visible {
                if row(ui, list, position, selected) {
                    clicked = Some(position);
                }
            }
        });
    clicked
}

fn row(ui: &mut Ui, list: &EntityList, position: usize, selected: Option<Guid>) -> bool {
    let Some(entity) = list.get(position) else {
        return false;
    };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let active = selected == Some(entity.guid());
    response.widget_info(|| WidgetInfo::selected(Role::Button, true, active, entity.class_name()));
    let state = RowState::of_list(active, response.hovered());
    paint_row(ui.painter(), rect, state);
    let (_, colour) = state.colours();
    let inner = Rect::from_min_max(
        pos2(rect.left() + TREE_PADDING_LEFT + SPACE_2, rect.top()),
        rect.max,
    );
    let name = line(
        ui,
        plain(entity.class_name(), ROW_FONT, row_weight(active)),
        (inner.width() - SPACE_2).max(0.0),
    );
    let name_right = inner.left() + name.size().x;
    paint_left(ui, inner, name, colour);
    let tail = inner.with_min_x(name_right + CHANNEL_GAP);
    let path = line(
        ui,
        plain(
            short_path(entity.record_path()),
            ROW_FONT,
            row_weight(false),
        ),
        (tail.width() - SPACE_2).max(0.0),
    );
    paint_left(ui, tail, path, palette::GRAY_2_TEXT);
    focus_inside(ui.painter(), rect, &response);
    tooltip::hover(response.clone(), |ui| {
        tooltip::line(ui, entity.record_path());
    });
    response.clicked()
}

/// Returns the record path below `entities/`, which tells the kind and the family apart.
fn short_path(path: &str) -> &str {
    path.split_once("/entities/").map_or(path, |(_, rest)| rest)
}
