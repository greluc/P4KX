// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use egui::scroll_area::ScrollAreaOutput;
use egui::{Align, Layout, Rect, Role, ScrollArea, Sense, Ui, WidgetInfo, pos2, vec2};
use egui_extras::{Column, TableBuilder, TableRow};

use super::localization_laid::{Cell, Laid};
use super::regions;
use super::table::head_fill;
use crate::theme::desktop::{
    CELL_PADDING_X, LOCALIZATION_KEY_WIDTH, LOCALIZATION_TEXT_WIDTH, QUERY_COLUMN_MIN_WIDTH,
    ROW_HEIGHT,
};
use crate::widgets::chip::{self, Tone};
use crate::widgets::rows::{RowState, focus_inside, paint_divider, paint_table_row};
use crate::widgets::scroll_region::{self, Stand};
use crate::{LocalizationTab, LocalizationTable, ScrollText, TextLanguage, Texts};

/// What the user did in the table.
pub(super) struct Picked {
    pub(super) row: u32,
    pub(super) language: Option<TextLanguage>,
}

/// Shows the rows the tab shows as a table with a key column and one column per language;
/// returns the cell that was clicked.
pub(super) fn show(
    ui: &mut Ui,
    texts: &Texts,
    tab: &LocalizationTab,
    table: &LocalizationTable,
    laid: &Laid,
) -> Option<Picked> {
    let hovered_id = ui.scope_id().with("localization-hovered-row");
    let hovered = ui.data(|data| data.get_temp::<usize>(hovered_id));
    let region = scroll_region::begin(
        ui,
        (ui.scope_id().with("localization-region"), || {
            texts.get(ScrollText::Localization)
        }),
        ROW_HEIGHT,
    );
    let wanted = region.requested();
    let mut area = ScrollArea::horizontal()
        .id_salt("localization")
        .auto_shrink(false);
    if let Some(wanted) = wanted {
        area = area.scroll_offset(vec2(wanted.x, 0.0));
    }
    let mut outcome = Outcome::default();
    let output = area.show(ui, |ui| {
        let parts = Parts { tab, table, laid };
        grid(
            ui,
            &parts,
            (wanted.map(|wanted| wanted.y), hovered),
            &mut outcome,
        )
    });
    let Outcome {
        picked,
        hovered: hovered_now,
    } = outcome;
    let rows = &output.inner;
    let stand = Stand {
        offset: vec2(
            output.state.clamped_offset().x,
            rows.state.clamped_offset().y,
        ),
        view: vec2(output.inner_rect.width(), rows.inner_rect.height()),
        content: vec2(output.content_size.x, rows.content_size.y),
    };
    region.finish(ui, output.inner_rect, stand);
    ui.data_mut(|data| {
        if let Some(index) = hovered_now {
            data.insert_temp(hovered_id, index);
        } else {
            data.remove_temp::<usize>(hovered_id);
        }
    });
    picked
}

#[derive(Default)]
struct Outcome {
    picked: Option<Picked>,
    hovered: Option<usize>,
}

#[derive(Clone, Copy)]
struct Parts<'a> {
    tab: &'a LocalizationTab,
    table: &'a LocalizationTable,
    laid: &'a Laid,
}

fn grid(
    ui: &mut Ui,
    parts: &Parts<'_>,
    (offset, hovered): (Option<f32>, Option<usize>),
    outcome: &mut Outcome,
) -> ScrollAreaOutput<()> {
    let Parts { tab, table, laid } = *parts;
    let columns = table.languages().len() + 1;
    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
    let left = ui.max_rect().left();
    let mut rows = builder(ui, columns);
    if let Some(offset) = offset {
        rows = rows.vertical_scroll_offset(offset);
    }
    rows.header(ROW_HEIGHT, |mut header| {
        for column in 0..columns {
            header.col(|ui| {
                let from = if column == 0 {
                    left
                } else {
                    ui.max_rect().left()
                };
                head_cell(ui, laid, column, from);
            });
        }
    })
    .body(|body| {
        regions::rows(body, ROW_HEIGHT, tab.shown_len(), |row| {
            let index = row.index();
            let Some(id) = tab.row_at(index) else {
                return;
            };
            let shown = Shown {
                tab,
                table,
                laid,
                hovered: hovered == Some(index),
                striped: index % 2 == 1,
            };
            let (click, is_hovered) = show_row(row, &shown, id);
            if click.is_some() {
                outcome.picked = click;
            }
            if is_hovered {
                outcome.hovered = Some(index);
            }
        });
    })
}

struct Shown<'a> {
    tab: &'a LocalizationTab,
    table: &'a LocalizationTable,
    laid: &'a Laid,
    hovered: bool,
    striped: bool,
}

fn builder(ui: &mut Ui, columns: usize) -> TableBuilder<'_> {
    let mut table = TableBuilder::new(ui)
        .id_salt("localization-rows")
        .striped(false)
        .resizable(true)
        .auto_shrink(false)
        .min_scrolled_height(0.0)
        .sense(Sense::hover())
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(
            Column::initial(LOCALIZATION_KEY_WIDTH)
                .at_least(QUERY_COLUMN_MIN_WIDTH)
                .clip(true),
        );
    for _ in 2..columns {
        table = table.column(
            Column::initial(LOCALIZATION_TEXT_WIDTH)
                .at_least(QUERY_COLUMN_MIN_WIDTH)
                .clip(true),
        );
    }
    table.column(
        Column::remainder()
            .at_least(QUERY_COLUMN_MIN_WIDTH)
            .clip(true),
    )
}

fn head_cell(ui: &Ui, laid: &Laid, column: usize, left: f32) {
    head_fill(ui, left);
    let Some((name, galley)) = laid.head(column) else {
        return;
    };
    let rect = ui.max_rect();
    let response = ui.interact(
        rect,
        ui.scope_id().with(("localization-head", column)),
        Sense::hover(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::ColumnHeader, true, &*name));
    let inner = rect.shrink2(vec2(CELL_PADDING_X, 0.0));
    ui.painter()
        .with_clip_rect(inner.intersect(ui.clip_rect()))
        .galley(
            pos2(inner.left(), inner.center().y - galley.size().y / 2.0),
            galley,
            crate::theme::palette::GRAY_1,
        );
}

fn show_row(mut row: TableRow<'_, '_>, shown: &Shown<'_>, id: u32) -> (Option<Picked>, bool) {
    row.set_hovered(false);
    let selected = shown.tab.selected();
    let on_row = selected.is_some_and(|(row, _)| row == id);
    let mut picked = None;
    let mut hovered = false;
    for column in 0..=shown.table.languages().len() {
        let language = column
            .checked_sub(1)
            .and_then(|index| shown.table.languages().get(index))
            .copied();
        let active = on_row && selected.is_some_and(|(_, chosen)| chosen == language);
        let state = if active {
            RowState::Active
        } else {
            RowState::of(false, on_row, shown.hovered)
        };
        row.col(|ui| {
            let response = cell(ui, shown, (id, column), state);
            hovered |= response.hovered();
            if response.clicked() {
                picked = Some(Picked { row: id, language });
            }
        });
    }
    (picked, hovered)
}

fn cell(ui: &Ui, shown: &Shown<'_>, (id, column): (u32, usize), state: RowState) -> egui::Response {
    let rect = ui.max_rect();
    let row_rect = rect.with_max_x(ui.clip_rect().right().max(rect.right()));
    paint_table_row(ui.painter(), row_rect, state, shown.striped);
    paint_divider(ui.painter(), row_rect);
    let response = ui.interact(
        rect,
        ui.scope_id().with(("localization-cell", id, column)),
        Sense::click(),
    );
    let inner = rect.shrink2(vec2(CELL_PADDING_X, 0.0));
    let content = shown.laid.cell(ui, shown.table, (id, column));
    let active = state == RowState::Active;
    match &*content {
        Cell::Text(galley) => {
            let galley = if active {
                shown
                    .laid
                    .active(ui, shown.table, (id, column))
                    .unwrap_or_else(|| Arc::clone(galley))
            } else {
                Arc::clone(galley)
            };
            response.widget_info(|| WidgetInfo::selected(Role::Cell, true, active, galley.text()));
            let painter = ui.painter().with_clip_rect(inner.intersect(ui.clip_rect()));
            let at = pos2(inner.left(), inner.center().y - galley.size().y / 2.0);
            painter.galley(at, galley, state.colours().1);
        }
        Cell::Missing => {
            if let Some((name, galley)) = shown.laid.missing() {
                response.widget_info(|| WidgetInfo::selected(Role::Cell, true, active, &*name));
                let size = galley.size();
                let left_centre = pos2(inner.left(), inner.center().y);
                if ui.is_rect_visible(Rect::from_min_size(left_centre, size)) {
                    chip::paint(ui, left_centre, galley, Tone::Warning);
                }
            }
        }
    }
    focus_inside(ui.painter(), rect, &response);
    response
}
