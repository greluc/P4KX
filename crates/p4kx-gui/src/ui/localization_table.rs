use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use egui::{Align, Galley, Id, Layout, Rect, Role, ScrollArea, Sense, Ui, WidgetInfo, pos2, vec2};
use egui_extras::{Column, TableBuilder, TableRow};

use super::diff_line::display;
use super::table::head_fill;
use crate::theme::desktop::{
    CELL_PADDING_X, LOCALIZATION_KEY_WIDTH, LOCALIZATION_TEXT_WIDTH, QUERY_COLUMN_MIN_WIDTH,
    ROW_FONT, ROW_HEIGHT, TABLE_HEAD_SIZE, TABLE_HEAD_TRACKING,
};
use crate::theme::tokens::{Weight, font};
use crate::widgets::chip::{self, Tone};
use crate::widgets::rows::{RowState, focus_inside, paint_divider, paint_row, row_weight};
use crate::widgets::text::{caps, line};
use crate::{LocalizationTab, LocalizationTable, LocalizationText, Text, TextLanguage, Texts};

const MAX_CACHED: usize = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Key {
    serial: u64,
    language: crate::Language,
    pixels_per_point: u32,
}

enum Cell {
    Missing,
    Text(Arc<Galley>),
}

struct Cache {
    key: Option<Key>,
    heads: Vec<(Arc<str>, Arc<Galley>)>,
    missing: Option<(Arc<str>, Arc<Galley>)>,
    cells: HashMap<(u32, usize), Arc<Cell>>,
}

/// The laid-out heads and cells of the table, kept until the shown rows, the table or the
/// interface language change.
#[derive(Clone)]
pub(super) struct Laid(Arc<Mutex<Cache>>);

impl Laid {
    pub(super) fn get(
        ui: &Ui,
        texts: &Texts,
        tab: &LocalizationTab,
        table: &LocalizationTable,
    ) -> Self {
        let id = Id::unique("p4kx-localization-table");
        let laid = ui.data_mut(|data| {
            data.get_temp_mut_or_insert_with(id, || {
                Self(Arc::new(Mutex::new(Cache {
                    key: None,
                    heads: Vec::new(),
                    missing: None,
                    cells: HashMap::new(),
                })))
            })
            .clone()
        });
        let key = Key {
            serial: tab.serial(),
            language: texts.language(),
            pixels_per_point: ui.ctx().pixels_per_point().to_bits(),
        };
        let mut cache = laid.lock();
        let stale = cache.key.is_none_or(|known| {
            known.language != key.language || known.pixels_per_point != key.pixels_per_point
        });
        if stale {
            let missing = texts.get(LocalizationText::Missing);
            *cache = Cache {
                key: Some(key),
                heads: heads(ui, texts, table),
                missing: Some((Arc::from(missing.as_str()), chip::galley(ui, &missing))),
                cells: HashMap::new(),
            };
        } else if cache.key != Some(key) || cache.cells.len() > MAX_CACHED {
            cache.key = Some(key);
            cache.cells.clear();
        }
        drop(cache);
        laid
    }

    fn lock(&self) -> MutexGuard<'_, Cache> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn cell(&self, ui: &Ui, table: &LocalizationTable, (row, column): (u32, usize)) -> Arc<Cell> {
        if let Some(cell) = self.lock().cells.get(&(row, column)) {
            return Arc::clone(cell);
        }
        let text = text_of(table, row, column);
        let cell = Arc::new(text.map_or(Cell::Missing, |text| {
            Cell::Text(layout(ui, text, Weight::Regular))
        }));
        self.lock().cells.insert((row, column), Arc::clone(&cell));
        cell
    }

    fn head(&self, column: usize) -> Option<(Arc<str>, Arc<Galley>)> {
        self.lock().heads.get(column).cloned()
    }

    fn missing(&self) -> Option<(Arc<str>, Arc<Galley>)> {
        self.lock().missing.clone()
    }
}

fn layout(ui: &Ui, text: &str, weight: Weight) -> Arc<Galley> {
    let job = display(text.as_bytes()).job(&font(ROW_FONT, weight));
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

fn heads(ui: &Ui, texts: &Texts, table: &LocalizationTable) -> Vec<(Arc<str>, Arc<Galley>)> {
    std::iter::once(texts.get(LocalizationText::ColumnKey))
        .chain(
            table
                .languages()
                .iter()
                .map(|&language| language_name(texts, language)),
        )
        .map(|name| {
            let text = caps(&name, TABLE_HEAD_SIZE, Weight::Bold, TABLE_HEAD_TRACKING);
            (Arc::from(name.as_str()), line(ui, text, f32::INFINITY))
        })
        .collect()
}

/// Returns the name of `language` in the interface language.
pub(super) fn language_name(texts: &Texts, language: TextLanguage) -> String {
    let known = LocalizationText::LanguageName(language);
    if known.id() == LocalizationText::OtherLanguage.id() {
        return format!(
            "{} ({})",
            texts.get(LocalizationText::OtherLanguage),
            language.code()
        );
    }
    texts.get(Text::Localization(known))
}

/// What the user did in the table.
pub(super) struct Picked {
    pub(super) row: u32,
    pub(super) language: Option<TextLanguage>,
}

/// Shows the rows the tab shows as a table with a key column and one column per language;
/// returns the cell that was clicked.
pub(super) fn show(
    ui: &mut Ui,
    tab: &LocalizationTab,
    table: &LocalizationTable,
    laid: &Laid,
) -> Option<Picked> {
    let columns = table.languages().len() + 1;
    let hovered_id = ui.scope_id().with("localization-hovered-row");
    let hovered = ui.data(|data| data.get_temp::<usize>(hovered_id));
    let mut hovered_now = None;
    let mut picked = None;
    ScrollArea::horizontal()
        .id_salt("localization")
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
                            head_cell(ui, laid, column, from);
                        });
                    }
                })
                .body(|body| {
                    body.rows(ROW_HEIGHT, tab.shown_len(), |row| {
                        let index = row.index();
                        let Some(id) = tab.row_at(index) else {
                            return;
                        };
                        let shown = Shown {
                            tab,
                            table,
                            laid,
                            hovered: hovered == Some(index),
                        };
                        let (click, is_hovered) = show_row(row, &shown, id);
                        if click.is_some() {
                            picked = click;
                        }
                        if is_hovered {
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
    picked
}

struct Shown<'a> {
    tab: &'a LocalizationTab,
    table: &'a LocalizationTable,
    laid: &'a Laid,
    hovered: bool,
}

fn builder(ui: &mut Ui, columns: usize) -> TableBuilder<'_> {
    let mut table = TableBuilder::new(ui)
        .id_salt("localization-rows")
        .striped(true)
        .resizable(true)
        .auto_shrink(false)
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
    paint_row(ui.painter(), row_rect, state);
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
                text_of(shown.table, id, column).map_or_else(
                    || Arc::clone(galley),
                    |text| layout(ui, text, row_weight(true)),
                )
            } else {
                Arc::clone(galley)
            };
            let name = galley.text().to_owned();
            response.widget_info(|| WidgetInfo::selected(Role::Cell, true, active, &name));
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

fn text_of(table: &LocalizationTable, id: u32, column: usize) -> Option<&str> {
    column.checked_sub(1).map_or_else(
        || table.key(id),
        |index| {
            table
                .languages()
                .get(index)
                .and_then(|&language| table.text(id, language))
        },
    )
}
