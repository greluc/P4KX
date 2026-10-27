// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align, Layout, Pos2, Rect, Response, Role, Sense, Ui, Vec2, WidgetInfo, pos2, vec2};
use egui_extras::{Column, TableBuilder, TableRow};

use super::record_rows::{Cell, RecordRowCache, kind_look, shown_path, shown_row};
use super::regions;
use super::table::{head_fill, sort_header};
use crate::theme::desktop::{CELL_PADDING_X, CHANGE_COLUMN, RECORD_TYPE_COLUMN, ROW_HEIGHT};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, Weight};
use crate::widgets::chip;
use crate::widgets::rows::{RowState, focus_inside, row_weight};
use crate::widgets::table_row::RowSlots;
use crate::widgets::text::{line, paint_left, plain};
use crate::widgets::tooltip;
use crate::{
    ComparisonText, DataForgeText, RecordChangeColumn, RecordChangeRow, RecordChangeText,
    RecordChangeView, Text, Texts,
};

const COLUMNS: [(RecordChangeColumn, Text); 3] = [
    (
        RecordChangeColumn::Change,
        Text::Comparison(ComparisonText::ColumnChange),
    ),
    (
        RecordChangeColumn::Path,
        Text::DataForge(DataForgeText::ColumnRecordPath),
    ),
    (
        RecordChangeColumn::Type,
        Text::DataForge(DataForgeText::ColumnType),
    ),
];

/// What the user did in the Record list.
pub(super) enum RecordAction {
    Sort(RecordChangeColumn),
    Select(u32),
}

/// Shows the sortable, virtual list of the shown Records of the Comparison `generation`,
/// scrolled so the row at `reveal` is visible; only the visible rows are laid out.
pub(super) fn show(
    ui: &mut Ui,
    texts: &Texts,
    (view, generation): (&RecordChangeView, u64),
    (filtered, reveal): (bool, Option<usize>),
) -> Option<RecordAction> {
    let mut cache = RecordRowCache::load(ui, texts, generation, &COLUMNS.map(|(_, text)| text));
    let hovered_id = ui.scope_id().with("records-hovered-row");
    let hovered = ui.data(|data| data.get_temp::<usize>(hovered_id));
    let mut hovered_now = None;
    let mut action = None;
    let left = ui.max_rect().left();
    let body_top = ui.cursor().top() + ROW_HEIGHT;
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    table(ui, reveal)
        .header(ROW_HEIGHT, |mut header| {
            for (index, (column, _)) in COLUMNS.into_iter().enumerate() {
                header.col(|ui| {
                    let from = if index == 0 {
                        left
                    } else {
                        ui.max_rect().left()
                    };
                    head_fill(ui, from);
                    let sort = view.sort();
                    let sorted = (sort.column == column).then_some(sort.direction);
                    if sort_header(ui, sorted, cache.head(index)) {
                        action = Some(RecordAction::Sort(column));
                    }
                });
            }
        })
        .body(|body| {
            regions::rows(body, ROW_HEIGHT, view.shown().len(), |row| {
                let index = row.index();
                let (clicked, is_hovered) =
                    show_row(row, (texts, view), &mut cache, hovered == Some(index));
                action = clicked.or_else(|| action.take());
                if is_hovered {
                    hovered_now = Some(index);
                }
            });
        });
    cache.store(ui);
    ui.data_mut(|data| match hovered_now {
        Some(index) => {
            data.insert_temp(hovered_id, index);
        }
        None => {
            data.remove_temp::<usize>(hovered_id);
        }
    });
    if view.shown().is_empty() && !view.is_filtering() {
        empty_note(ui, texts, filtered, pos2(left, body_top));
    }
    action
}

fn table(ui: &mut Ui, reveal: Option<usize>) -> TableBuilder<'_> {
    let mut table = TableBuilder::new(ui);
    if let Some(row) = reveal {
        table = table.scroll_to_row(row, None);
    }
    table
        .id_salt("comparison-records")
        .striped(false)
        .resizable(true)
        .auto_shrink(false)
        .min_scrolled_height(0.0)
        .sense(Sense::click())
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(Column::exact(CHANGE_COLUMN))
        .column(Column::remainder().at_least(160.0).clip(true))
        .column(
            Column::initial(RECORD_TYPE_COLUMN)
                .at_least(80.0)
                .clip(true),
        )
}

fn show_row(
    mut row: TableRow<'_, '_>,
    (texts, view): (&Texts, &RecordChangeView),
    cache: &mut RecordRowCache,
    hovered: bool,
) -> (Option<RecordAction>, bool) {
    row.set_hovered(false);
    let Some((id, record)) = shown_row(view, row.index()) else {
        return (None, false);
    };
    let active = view.selected() == Some(id);
    let state = RowState::of(active, false, hovered);
    let (_, colour) = state.colours();
    let weight = row_weight(active);
    let index = row.index();
    let mut slots = None;
    row.col(|ui| {
        slots = Some(RowSlots::reserve(ui));
        let rect = ui.max_rect();
        if let Some(label) = cache.chip(record.kind()) {
            let at = pos2(rect.left() + CELL_PADDING_X, rect.center().y);
            chip::paint(ui, at, label, kind_look(record.kind()).1);
        }
    });
    for cell in [Cell::Path, Cell::Type] {
        row.col(|ui| {
            let rect = ui.max_rect().shrink2(vec2(CELL_PADDING_X, 0.0));
            cache
                .cell(ui, (id, record, cell), (rect.width(), weight))
                .paint(ui, rect, colour);
        });
    }
    let response = row.response();
    if let Some(slots) = slots {
        slots.paint(&response, state, index % 2 == 1);
    }
    let is_hovered = response.hovered();
    let clicked = finish_row(&response, texts, record, active);
    (clicked.then_some(RecordAction::Select(id)), is_hovered)
}

/// Names the row of `record` for assistive technology, rings it while focused and shows the
/// record paths and type on hover; returns whether it was clicked.
fn finish_row(response: &Response, texts: &Texts, record: &RecordChangeRow, active: bool) -> bool {
    response.widget_info(|| {
        let kind = texts.get(kind_look(record.kind()).0);
        let name = texts.format(
            ComparisonText::Row,
            &[
                ("change", kind.into()),
                ("path", shown_path(texts, record).into()),
            ],
        );
        WidgetInfo::selected(Role::Row, true, active, name)
    });
    focus_inside(
        &response.ctx.layer_painter(response.layer_id),
        response.rect,
        response,
    );
    tooltip::hover(response.clone(), |ui| {
        tooltip::line(ui, shown_path(texts, record));
        tooltip::line(ui, record.type_name());
    })
    .clicked()
}

fn empty_note(ui: &Ui, texts: &Texts, filtered: bool, top_left: Pos2) {
    let text = if filtered {
        Text::Comparison(ComparisonText::NoMatches)
    } else {
        Text::RecordChange(RecordChangeText::NoDifferences)
    };
    let rect = Rect::from_min_size(top_left, vec2(ui.max_rect().width(), ROW_HEIGHT))
        .shrink2(vec2(CELL_PADDING_X, 0.0));
    let galley = line(
        ui,
        plain(texts.get(text), FS_XS, Weight::Regular),
        rect.width(),
    );
    let response = ui.interact(rect, ui.scope_id().with("records-empty"), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, texts.get(text)));
    paint_left(ui, rect, galley, palette::GRAY_2_TEXT);
}
