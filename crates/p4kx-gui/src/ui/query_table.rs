use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use egui::{Align, Galley, Id, Layout, Role, ScrollArea, Sense, Ui, WidgetInfo, vec2};
use egui_extras::{Column, TableBuilder, TableRow};

use super::table::{head_fill, sort_head};
use crate::theme::desktop::{
    CELL_PADDING_X, QUERY_COLUMN_MIN_WIDTH, QUERY_COLUMN_WIDTH, ROW_FONT, ROW_HEIGHT,
    TABLE_HEAD_SIZE, TABLE_HEAD_TRACKING,
};
use crate::theme::palette;
use crate::theme::tokens::Weight;
use crate::widgets::rows::{RowState, paint_divider, paint_row};
use crate::widgets::text::{caps, line, paint_left, plain};
use crate::{DataForgeText, Language, QueryResult, Texts};

const MAX_CACHED: usize = 8192;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Key {
    generation: u64,
    language: Language,
    pixels_per_point: f32,
}

type Labelled = (Arc<str>, Arc<Galley>);

#[derive(Default)]
struct Cache {
    key: Option<Key>,
    heads: Vec<Labelled>,
    cells: HashMap<(usize, usize), (u32, Arc<Galley>)>,
}

/// The laid-out heads and cells of the result table, kept until the shown rows or the language
/// change; a cell is laid out again when its column changes width.
#[derive(Clone)]
pub(super) struct Laid(Arc<Mutex<Cache>>);

impl Laid {
    pub(super) fn get(ui: &Ui, texts: &Texts, result: &QueryResult) -> Self {
        let id = Id::unique("p4kx-query-table");
        let laid = ui.data_mut(|data| {
            data.get_temp_mut_or_insert_with(id, || Self(Arc::new(Mutex::new(Cache::default()))))
                .clone()
        });
        let key = Key {
            generation: result.generation(),
            language: texts.language(),
            pixels_per_point: ui.ctx().pixels_per_point(),
        };
        let mut cache = laid.lock();
        if cache.key != Some(key) || cache.cells.len() > MAX_CACHED {
            *cache = Cache {
                key: Some(key),
                heads: heads(ui, texts, result),
                cells: HashMap::new(),
            };
        }
        drop(cache);
        laid
    }

    fn lock(&self) -> MutexGuard<'_, Cache> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn cell(
        &self,
        ui: &Ui,
        (position, column): (usize, usize),
        text: &str,
        width: f32,
    ) -> Arc<Galley> {
        let width_key = width.to_bits();
        let known = self
            .lock()
            .cells
            .get(&(position, column))
            .filter(|(known, _)| *known == width_key)
            .map(|(_, galley)| Arc::clone(galley));
        if let Some(galley) = known {
            return galley;
        }
        let galley = line(ui, plain(text, ROW_FONT, Weight::Regular), width);
        self.lock()
            .cells
            .insert((position, column), (width_key, Arc::clone(&galley)));
        galley
    }

    fn head(&self, column: usize) -> Option<Labelled> {
        self.lock().heads.get(column).cloned()
    }
}

/// Returns the heads: the record path as a label in upper case, then each selected path as
/// written, since a path is data.
fn heads(ui: &Ui, texts: &Texts, result: &QueryResult) -> Vec<Labelled> {
    let record_path = texts.get(DataForgeText::ColumnRecordPath);
    let first = caps(
        &record_path,
        TABLE_HEAD_SIZE,
        Weight::Bold,
        TABLE_HEAD_TRACKING,
    );
    std::iter::once((record_path.into(), line(ui, first, f32::INFINITY)))
        .chain(result.columns().iter().map(|column| {
            let text = plain(column.as_str(), TABLE_HEAD_SIZE, Weight::Bold);
            (Arc::from(column.as_str()), line(ui, text, f32::INFINITY))
        }))
        .collect()
}

/// Shows the page of the result as a table with one column per selected path, sortable by
/// every column; returns the column whose head was clicked.
pub(super) fn show(ui: &mut Ui, result: &QueryResult, laid: &Laid) -> Option<usize> {
    let columns = result.columns().len() + 1;
    let page = result.page_rows();
    let hovered_id = ui.scope_id().with("query-hovered-row");
    let hovered = ui.data(|data| data.get_temp::<usize>(hovered_id));
    let mut hovered_now = None;
    let mut sort = None;
    ScrollArea::horizontal()
        .id_salt("query-result")
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            let left = ui.max_rect().left();
            builder(ui, columns)
                .header(ROW_HEIGHT, |mut header| {
                    for column in 0..columns {
                        header.col(|ui| {
                            let from = if column == 0 {
                                left
                            } else {
                                ui.max_rect().left()
                            };
                            if head_cell(ui, (result, laid), column, from) {
                                sort = Some(column);
                            }
                        });
                    }
                })
                .body(|body| {
                    body.rows(ROW_HEIGHT, page.len(), |row| {
                        let index = row.index();
                        let position = page.start + index;
                        if show_row(row, (result, laid), position, hovered == Some(index)) {
                            hovered_now = Some(index);
                        }
                    });
                });
        });
    ui.data_mut(|data| {
        if let Some(index) = hovered_now {
            data.insert_temp(hovered_id, index);
        } else {
            data.remove_temp::<usize>(hovered_id);
        }
    });
    sort
}

fn builder(ui: &mut Ui, columns: usize) -> TableBuilder<'_> {
    let mut table = TableBuilder::new(ui)
        .id_salt("query-rows")
        .striped(true)
        .resizable(true)
        .auto_shrink(false)
        .sense(Sense::hover())
        .cell_layout(Layout::left_to_right(Align::Center));
    for _ in 1..columns {
        table = table.column(
            Column::initial(QUERY_COLUMN_WIDTH)
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

/// Shows the head of `column`, filled from `left`; returns whether it was clicked.
fn head_cell(ui: &Ui, (result, laid): (&QueryResult, &Laid), column: usize, left: f32) -> bool {
    head_fill(ui, left);
    let sorted = result
        .sort()
        .filter(|&(by, _)| by == column)
        .map(|(_, direction)| direction);
    laid.head(column)
        .is_some_and(|(name, galley)| sort_head(ui, sorted, &name, &galley))
}
fn show_row(
    mut row: TableRow<'_, '_>,
    (result, laid): (&QueryResult, &Laid),
    position: usize,
    hovered: bool,
) -> bool {
    row.set_hovered(false);
    let state = RowState::of(false, false, hovered);
    let colour = state.colours().1;
    for column in 0..=result.columns().len() {
        row.col(|ui| {
            let rect = ui.max_rect();
            let row_rect = rect.with_max_x(ui.clip_rect().right().max(rect.right()));
            paint_row(ui.painter(), row_rect, state);
            paint_divider(ui.painter(), row_rect);
            let text = result.cell(position, column).unwrap_or_default();
            let inner = rect.shrink2(vec2(CELL_PADDING_X, 0.0));
            let response = ui.interact(
                inner,
                ui.scope_id().with(("query-cell", position, column)),
                Sense::hover(),
            );
            response.widget_info(|| WidgetInfo::labeled(Role::Label, true, text));
            let galley = laid.cell(ui, (position, column), text, inner.width());
            let colour = if result.is_error(position, column) {
                palette::DANGER_TEXT
            } else {
                colour
            };
            paint_left(ui, inner, galley, colour);
        });
    }
    row.response().hovered()
}
