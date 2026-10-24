// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{
    Align, Color32, CornerRadius, Layout, Rect, Response, Role, ScrollArea, Sense, Ui, Vec2,
    WidgetInfo, pos2, vec2,
};
use egui_extras::{Column, TableBuilder, TableRow};

use super::table::head_fill;
use super::tree::{rails, toggle};
use crate::theme::desktop::{
    CELL_PADDING_X, ROW_FONT, ROW_HEIGHT, TABLE_HEAD_SIZE, TABLE_HEAD_TRACKING,
};
use crate::theme::palette;
use crate::theme::tags_metrics::{TAGS_GUID_WIDTH, TAGS_LEGACY_WIDTH, TAGS_NAME_MIN_WIDTH};
use crate::theme::tokens::Weight;
use crate::widgets::rows::{RowState, focus_inside, paint_divider, paint_row, row_weight};
use crate::widgets::text::{caps, line, paint_left, paint_right, plain};
use crate::widgets::tooltip;
use crate::{TagRow, TagsText, TagsView, Texts};

/// What the user did in the tree: the position in the pre-order of the tag it is about.
#[derive(Clone, Copy)]
pub(super) enum Action {
    Select(usize),
    Toggle(usize),
}

/// The heads of the columns, left to right.
const HEADS: [TagsText; 3] = [
    TagsText::ColumnTag,
    TagsText::ColumnGuid,
    TagsText::ColumnLegacy,
];

#[derive(Default)]
struct Outcome {
    action: Option<Action>,
    hovered: Option<usize>,
}

/// What a cell of a row shows: the row, its place among the rows shown, its column and its state.
struct Cell<'a, 'v> {
    row: &'a TagRow<'v>,
    index: usize,
    column: usize,
    state: RowState,
}

/// Shows the tag tree as a table with the tag, GUID and legacy ID columns, as wide as its columns
/// need and scrolling sideways when the window is narrower; returns what the user did.
pub(super) fn show(ui: &mut Ui, texts: &Texts, view: &TagsView) -> Option<Action> {
    let hovered_id = ui.scope_id().with("tags-hovered-row");
    let hovered = ui.data(|data| data.get_temp::<usize>(hovered_id));
    let mut outcome = Outcome::default();
    ScrollArea::horizontal()
        .id_salt("tags")
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let left = ui.max_rect().left();
            TableBuilder::new(ui)
                .id_salt("tags-rows")
                .striped(false)
                .resizable(false)
                .auto_shrink(false)
                .sense(Sense::hover())
                .cell_layout(Layout::left_to_right(Align::Center))
                .column(Column::remainder().at_least(TAGS_NAME_MIN_WIDTH).clip(true))
                .column(Column::exact(TAGS_GUID_WIDTH).clip(true))
                .column(Column::exact(TAGS_LEGACY_WIDTH).clip(true))
                .header(ROW_HEIGHT, |mut header| {
                    for (column, text) in HEADS.into_iter().enumerate() {
                        header.col(|ui| head_cell(ui, texts, (column, text), left));
                    }
                })
                .body(|body| {
                    body.rows(ROW_HEIGHT, view.shown_len(), |row| {
                        let index = row.index();
                        let Some(tag) = view.row(index) else {
                            return;
                        };
                        let (action, is_hovered) =
                            show_row(row, texts, (&tag, index), hovered == Some(index));
                        if action.is_some() {
                            outcome.action = action;
                        }
                        if is_hovered {
                            outcome.hovered = Some(index);
                        }
                    });
                });
        });
    ui.data_mut(|data| {
        if let Some(index) = outcome.hovered {
            data.insert_temp(hovered_id, index);
        } else {
            data.remove_temp::<usize>(hovered_id);
        }
    });
    outcome.action
}

/// Paints the head cell of `column`: the fill and the orange rule, the label in capitals; the
/// legacy ID is a number and its head is right-aligned.
fn head_cell(ui: &Ui, texts: &Texts, (column, text): (usize, TagsText), table_left: f32) {
    let rect = ui.max_rect();
    head_fill(ui, if column == 0 { table_left } else { rect.left() });
    let name = texts.get(text);
    let inner = rect.shrink2(vec2(CELL_PADDING_X, 0.0));
    let galley = line(
        ui,
        caps(&name, TABLE_HEAD_SIZE, Weight::Bold, TABLE_HEAD_TRACKING),
        inner.width().max(0.0),
    );
    if text == TagsText::ColumnLegacy {
        paint_right(ui, inner, galley, palette::GRAY_1);
    } else {
        paint_left(ui, inner, galley, palette::GRAY_1);
    }
    ui.interact(
        rect,
        ui.scope_id().with(("tags-head", column)),
        Sense::hover(),
    )
    .widget_info(|| WidgetInfo::labeled(Role::ColumnHeader, true, &name));
}

fn show_row(
    mut row: TableRow<'_, '_>,
    texts: &Texts,
    (tag_row, index): (&TagRow<'_>, usize),
    hovered: bool,
) -> (Option<Action>, bool) {
    row.set_hovered(false);
    let position = tag_row.tag().position();
    let state = RowState::of_tree(tag_row.is_selected(), hovered);
    let mut action = None;
    let mut is_hovered = false;
    for column in 0..HEADS.len() {
        row.col(|ui| {
            let cell = Cell {
                row: tag_row,
                index,
                column,
                state,
            };
            let (response, toggled) = cell.show(ui, texts);
            is_hovered |= response.hovered();
            if toggled || (response.double_clicked() && tag_row.is_expandable()) {
                action = Some(Action::Toggle(position));
            } else if response.clicked() && action.is_none() {
                action = Some(Action::Select(position));
            }
        });
    }
    (action, is_hovered)
}

impl Cell<'_, '_> {
    /// Shows the cell; returns its response and whether its chevron was clicked.
    fn show(&self, ui: &Ui, texts: &Texts) -> (Response, bool) {
        let tag = self.row.tag();
        let rect = ui.max_rect();
        if self.column == 0 {
            paint_row(ui.painter(), rect, self.state);
        } else {
            ui.painter()
                .rect_filled(rect, CornerRadius::ZERO, self.state.colours().0);
        }
        paint_divider(ui.painter(), rect);
        let id = ui
            .scope_id()
            .with(("tags-cell", tag.position(), self.column));
        let response = ui.interact(rect, id, Sense::click());
        let (_, colour) = self.state.colours();
        let muted = if self.row.is_selected() {
            colour
        } else {
            palette::GRAY_2_TEXT
        };
        let toggled = match self.column {
            0 => self.name(ui, texts, &response, colour),
            1 => {
                let text = tag.guid().to_string();
                number_or_id(ui, &response, text, muted, false);
                false
            }
            _ => {
                let text = tag
                    .legacy_id()
                    .map_or_else(String::new, |id| id.to_string());
                number_or_id(ui, &response, text, muted, true);
                false
            }
        };
        focus_inside(ui.painter(), rect, &response);
        (response, toggled)
    }

    /// Shows the chevron, the rails and the name of the first cell; returns whether the chevron
    /// was clicked.
    fn name(&self, ui: &Ui, texts: &Texts, response: &Response, colour: Color32) -> bool {
        let tag = self.row.tag();
        let rect = ui.max_rect();
        response.widget_info(|| {
            WidgetInfo::selected(Role::Button, true, self.row.is_selected(), tag.path())
        });
        let chevron = rails(ui, rect, tag.depth());
        let toggled = self.row.is_expandable() && {
            let id = ui
                .scope_id()
                .with(("tags-toggle", tag.position(), self.index));
            toggle(ui, texts, (id, chevron), self.row.is_expanded(), tag.name())
        };
        let name_rect = Rect::from_min_max(pos2(chevron.right(), rect.top()), rect.max);
        let galley = line(
            ui,
            plain(tag.name(), ROW_FONT, row_weight(self.row.is_selected())),
            name_rect.width().max(0.0),
        );
        paint_left(ui, name_rect, galley, colour);
        tooltip::hover(response.clone(), |ui| {
            tooltip::line(ui, tag.path());
        });
        toggled
    }
}

/// Paints `text` in the cell of `ui`, right-aligned when it is a number, and names the cell for
/// assistive technology.
fn number_or_id(ui: &Ui, response: &Response, text: String, colour: Color32, right: bool) {
    let inner = ui.max_rect().shrink2(vec2(CELL_PADDING_X, 0.0));
    response.widget_info(|| WidgetInfo::labeled(Role::Cell, true, &text));
    let galley = line(
        ui,
        plain(text, ROW_FONT, Weight::Regular),
        inner.width().max(0.0),
    );
    if right {
        paint_right(ui, inner, galley, colour);
    } else {
        paint_left(ui, inner, galley, colour);
    }
}
