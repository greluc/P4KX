// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::scroll_area::ScrollAreaOutput;
use egui::{Align, Color32, Layout, Role, Sense, Ui, Vec2, WidgetInfo, pos2, vec2};
use egui_extras::{Column, TableBuilder, TableRow};

use super::regions;
use super::table::{cell_text, head_fill};
use crate::theme::desktop::{CELL_PADDING_X, ROW_HEIGHT, TABLE_HEAD_SIZE, TABLE_HEAD_TRACKING};
use crate::theme::palette;
use crate::theme::sound_metrics::SOUND_COLUMN_MIN;
use crate::theme::tokens::Weight;
use crate::widgets::rows::{RowState, focus_inside, row_weight};
use crate::widgets::scroll_region::{self, Stand};
use crate::widgets::table_row::RowSlots;
use crate::widgets::text::{caps, line};

/// How wide a column starts.
#[derive(Debug, Clone, Copy)]
pub(super) enum Width {
    Initial(f32),
    Rest,
}

/// How a cell is tinted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Tint {
    Normal,
    Muted,
    Warning,
    Success,
}

/// The text of one cell.
#[derive(Debug, Clone)]
pub(super) struct Cell {
    pub(super) text: String,
    pub(super) tint: Tint,
}

impl Cell {
    pub(super) const fn new(text: String, tint: Tint) -> Self {
        Self { text, tint }
    }
}

/// The cells of one row and its accessible name.
#[derive(Debug, Clone)]
pub(super) struct Row {
    pub(super) cells: Vec<Cell>,
    pub(super) name: String,
}

/// The laid-out text of a grid, formatted once per list and language.
#[derive(Debug)]
pub(super) struct Laid {
    pub(super) heads: Vec<String>,
    pub(super) rows: Vec<Row>,
}

/// What a grid shows.
pub(super) struct Grid<'a> {
    pub(super) salt: &'static str,
    pub(super) label: String,
    pub(super) widths: &'a [Width],
    pub(super) laid: &'a Laid,
    pub(super) selected: Option<usize>,
    pub(super) height: Option<f32>,
}

#[derive(Default)]
struct Outcome {
    clicked: Option<usize>,
    hovered: Option<usize>,
}

const fn colour(tint: Tint, normal: Color32) -> Color32 {
    match tint {
        Tint::Normal => normal,
        Tint::Muted => palette::GRAY_2_TEXT,
        Tint::Warning => palette::WARNING,
        Tint::Success => palette::SUCCESS_TEXT,
    }
}

/// Shows the rows of `grid` as a virtual table; returns the position of the row that was
/// clicked. The table has a scope of its own, so that its column dividers end with its rows and
/// do not run on through the footer of the list above it.
pub(super) fn show(ui: &mut Ui, grid: &Grid<'_>) -> Option<usize> {
    ui.scope(|ui| show_table(ui, grid)).inner
}

fn show_table(ui: &mut Ui, grid: &Grid<'_>) -> Option<usize> {
    let region = scroll_region::begin(
        ui,
        (ui.scope_id().with((grid.salt, "region")), || {
            grid.label.clone()
        }),
        ROW_HEIGHT,
    );
    let hovered_id = ui.scope_id().with((grid.salt, "hovered-row"));
    let hovered = ui.data(|data| data.get_temp::<usize>(hovered_id));
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    let left = ui.max_rect().left();
    let mut table = builder(ui, grid);
    if let Some(offset) = region.requested() {
        table = table.vertical_scroll_offset(offset.y);
    }
    let mut outcome = Outcome::default();
    let output = body(table, grid, (left, hovered), &mut outcome);
    let stand = Stand {
        offset: output.state.clamped_offset(),
        view: output.inner_rect.size(),
        content: output.content_size,
    };
    region.finish(ui, output.inner_rect, stand);
    ui.data_mut(|data| match outcome.hovered {
        Some(index) => {
            data.insert_temp(hovered_id, index);
        }
        None => {
            data.remove_temp::<usize>(hovered_id);
        }
    });
    outcome.clicked
}

fn builder<'a>(ui: &'a mut Ui, grid: &Grid<'_>) -> TableBuilder<'a> {
    let mut table = TableBuilder::new(ui)
        .id_salt(grid.salt)
        .striped(false)
        .resizable(true)
        .auto_shrink(false)
        .min_scrolled_height(0.0)
        .sense(Sense::click())
        .cell_layout(Layout::left_to_right(Align::Center));
    if let Some(height) = grid.height {
        table = table.min_scrolled_height(height).max_scroll_height(height);
    }
    for width in grid.widths {
        table = table.column(match *width {
            Width::Initial(width) => Column::initial(width).at_least(SOUND_COLUMN_MIN).clip(true),
            Width::Rest => Column::remainder().at_least(SOUND_COLUMN_MIN).clip(true),
        });
    }
    table
}

fn body(
    table: TableBuilder<'_>,
    grid: &Grid<'_>,
    (left, hovered): (f32, Option<usize>),
    outcome: &mut Outcome,
) -> ScrollAreaOutput<()> {
    table
        .header(ROW_HEIGHT, |mut header| {
            for (index, label) in grid.laid.heads.iter().enumerate() {
                header.col(|ui| {
                    let from = if index == 0 {
                        left
                    } else {
                        ui.max_rect().left()
                    };
                    head_cell(ui, grid.salt, label, from);
                });
            }
        })
        .body(|body| {
            regions::rows(body, ROW_HEIGHT, grid.laid.rows.len(), |row| {
                let index = row.index();
                let Some(texts) = grid.laid.rows.get(index) else {
                    return;
                };
                let active = grid.selected == Some(index);
                let state = RowState::of(active, false, hovered == Some(index));
                let (picked, is_hovered) = show_row(row, texts, state);
                if picked {
                    outcome.clicked = Some(index);
                }
                if is_hovered {
                    outcome.hovered = Some(index);
                }
            });
        })
}

fn head_cell(ui: &Ui, salt: &'static str, label: &str, left: f32) {
    head_fill(ui, left);
    let rect = ui.max_rect();
    let response = ui.interact(
        rect,
        ui.scope_id().with((salt, "head", label)),
        Sense::hover(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::ColumnHeader, true, label));
    let inner = rect.shrink2(vec2(CELL_PADDING_X, 0.0));
    let text = caps(label, TABLE_HEAD_SIZE, Weight::Bold, TABLE_HEAD_TRACKING);
    let galley = line(ui, text, inner.width());
    ui.painter()
        .with_clip_rect(inner.intersect(ui.clip_rect()))
        .galley(
            pos2(inner.left(), inner.center().y - galley.size().y / 2.0),
            galley,
            palette::GRAY_1,
        );
}

fn show_row(mut row: TableRow<'_, '_>, texts: &Row, state: RowState) -> (bool, bool) {
    row.set_hovered(false);
    let (_, normal) = state.colours();
    let active = state == RowState::Active;
    let weight = row_weight(active);
    let index = row.index();
    let mut slots = None;
    for (position, cell) in texts.cells.iter().enumerate() {
        row.col(|ui| {
            if position == 0 {
                slots = Some(RowSlots::reserve(ui));
            }
            cell_text(ui, &cell.text, colour(cell.tint, normal), weight);
        });
    }
    let response = row.response();
    if let Some(slots) = slots {
        slots.paint(&response, state, index % 2 == 1);
    }
    response.widget_info(|| WidgetInfo::selected(Role::Row, true, active, &texts.name));
    focus_inside(
        &response.ctx.layer_painter(response.layer_id),
        response.rect,
        &response,
    );
    (response.clicked(), response.hovered())
}

/// Returns the laid-out text cached under `salt` for `serial` and the language `key`, laying it
/// out with `lay` when it is missing or stale.
pub(super) fn cached<K: Copy + PartialEq + Send + Sync + 'static>(
    ui: &Ui,
    salt: &'static str,
    key: K,
    lay: impl FnOnce() -> Laid,
) -> std::sync::Arc<Laid> {
    let id = ui.scope_id().with(("p4kx-sound-grid-texts", salt));
    if let Some((cached, laid)) = ui.data(|data| data.get_temp::<(K, std::sync::Arc<Laid>)>(id))
        && cached == key
    {
        return laid;
    }
    let laid = std::sync::Arc::new(lay());
    ui.data_mut(|data| data.insert_temp(id, (key, std::sync::Arc::clone(&laid))));
    laid
}
