// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Color32, CornerRadius, Galley, Id, Rect, Role, ScrollArea, Sense, Ui, WidgetInfo, pos2, vec2,
};
use p4kx_search::{Excerpt, Source, display_text};

use crate::texts::language_name;
use crate::theme::desktop::{
    CELL_PADDING_X, ROW_FONT, ROW_HEIGHT, TABLE_HEAD_SIZE, TABLE_HEAD_TRACKING,
};
use crate::theme::palette;
use crate::theme::tokens::{Weight, font};
use crate::widgets::rows::{RowState, focus_inside, paint_divider, paint_table_row};
use crate::widgets::scroll_region;
use crate::widgets::text::{caps, line, paint_left, plain};
use crate::{ContentResult, ContentSearchTab, ContentSearchText, ScrollText, Texts, format};

const MIN_FIRST: f32 = 160.0;
const MIN_SECOND: f32 = 110.0;

/// One row of the list: three cells, the last an excerpt with its match.
struct Cells<'a> {
    first: String,
    second: String,
    excerpt: &'a Excerpt,
}

/// A row laid out once: the cells in regular weight, in bold once the row was selected, and the
/// excerpt, whose unmatched text takes the colour the row is painted in.
struct Laid {
    cells: [Arc<Galley>; 2],
    bold: Option<[Arc<Galley>; 2]>,
    excerpt: Arc<Galley>,
}

/// The laid-out heads and rows of the listed source, made again only when the list, the
/// language, the width or the scale change; rows are laid out as they come into view.
struct Cache {
    key: u64,
    heads: [(Arc<str>, Arc<Galley>); 3],
    row_width: f32,
    rows: Vec<Option<Laid>>,
}

/// Shows the hits of the listed source as rows of three columns and returns the position of the
/// hit that was clicked.
pub(super) fn show(
    ui: &mut Ui,
    texts: &Texts,
    result: &ContentResult,
    tab: &ContentSearchTab,
) -> Option<usize> {
    let source = tab.shown();
    let total = result.len(source);
    let width = ui.available_width();
    let widths = widths(width);
    let cache = cache(ui, texts, tab, total, width);
    head(ui, &lock(&cache).heads, widths);
    let hovered_id = ui.scope_id().with("content-search-hovered");
    let hovered = ui.data(|data| data.get_temp::<usize>(hovered_id));
    let mut clicked = None;
    let mut hovered_now = None;
    let area = ScrollArea::vertical()
        .id_salt(("content-search-hits", tab.serial()))
        .auto_shrink(false);
    scroll_region::show(
        ui,
        (ui.scope_id().with("content-search-region"), || {
            texts.get(ScrollText::Hits)
        }),
        ROW_HEIGHT,
        area,
        |area, ui| {
            area.show_rows(ui, ROW_HEIGHT, total, |ui, range| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                let mut cache = lock(&cache);
                for position in range {
                    let row = Row {
                        position,
                        active: tab.selected() == Some(position),
                        hovered: hovered == Some(position),
                        widths,
                    };
                    let hit = (texts, result, source);
                    let Some(laid) = laid(ui, hit, &mut cache, &row) else {
                        continue;
                    };
                    let (was_clicked, is_hovered) = show_row(ui, hit, laid, &row);
                    if was_clicked {
                        clicked = Some(position);
                    }
                    if is_hovered {
                        hovered_now = Some(position);
                    }
                }
            })
        },
    );
    ui.data_mut(|data| match hovered_now {
        Some(position) => {
            data.insert_temp(hovered_id, position);
        }
        None => {
            data.remove_temp::<usize>(hovered_id);
        }
    });
    clicked
}

fn lock(cache: &Mutex<Cache>) -> MutexGuard<'_, Cache> {
    cache.lock().unwrap_or_else(PoisonError::into_inner)
}

fn cache(
    ui: &Ui,
    texts: &Texts,
    tab: &ContentSearchTab,
    total: usize,
    width: f32,
) -> Arc<Mutex<Cache>> {
    let mut hasher = DefaultHasher::new();
    texts.language().hash(&mut hasher);
    tab.serial().hash(&mut hasher);
    width.to_bits().hash(&mut hasher);
    ui.ctx().pixels_per_point().to_bits().hash(&mut hasher);
    let key = hasher.finish();
    let id = Id::unique("p4kx-content-search-rows");
    if let Some(cache) = ui.data(|data| data.get_temp::<Arc<Mutex<Cache>>>(id))
        && lock(&cache).key == key
    {
        return cache;
    }
    let cache = Arc::new(Mutex::new(Cache {
        key,
        heads: heads(ui, texts, tab.shown(), width),
        row_width: f32::NAN,
        rows: std::iter::repeat_with(|| None).take(total).collect(),
    }));
    ui.data_mut(|data| data.insert_temp(id, Arc::clone(&cache)));
    cache
}

fn heads(ui: &Ui, texts: &Texts, source: Source, width: f32) -> [(Arc<str>, Arc<Galley>); 3] {
    let (first, second) = match source {
        Source::Records => (
            ContentSearchText::ColumnRecord,
            ContentSearchText::ColumnField,
        ),
        Source::Localization => (
            ContentSearchText::ColumnLanguage,
            ContentSearchText::ColumnKey,
        ),
        Source::Entries => (
            ContentSearchText::ColumnEntry,
            ContentSearchText::ColumnLine,
        ),
    };
    let row = Rect::from_min_size(pos2(0.0, 0.0), vec2(width, ROW_HEIGHT));
    [(0, first), (1, second), (2, ContentSearchText::ColumnText)].map(|(index, text)| {
        let name = texts.get(text);
        let column = column_rect(row, widths(width), index);
        let galley = line(
            ui,
            caps(&name, TABLE_HEAD_SIZE, Weight::Bold, TABLE_HEAD_TRACKING),
            column.width(),
        );
        (Arc::<str>::from(name), galley)
    })
}

struct Row {
    position: usize,
    active: bool,
    hovered: bool,
    widths: [f32; 2],
}

fn widths(available: f32) -> [f32; 2] {
    [
        (available * 0.3).clamp(MIN_FIRST, 420.0),
        (available * 0.18).clamp(MIN_SECOND, 300.0),
    ]
}

fn head(ui: &mut Ui, heads: &[(Arc<str>, Arc<Galley>); 3], widths: [f32; 2]) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette::SURFACE_INPUT);
    let rule = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), rect.max);
    ui.painter()
        .rect_filled(rule, CornerRadius::ZERO, palette::PRIMARY);
    for (index, (name, galley)) in heads.iter().enumerate() {
        let column = column_rect(rect, widths, index);
        let response = ui.interact(
            column,
            ui.scope_id().with(("content-search-head", index)),
            Sense::hover(),
        );
        response.widget_info(|| WidgetInfo::labeled(Role::ColumnHeader, true, &**name));
        paint_left(ui, column, Arc::clone(galley), palette::GRAY_1);
    }
}

fn column_rect(row: Rect, widths: [f32; 2], index: usize) -> Rect {
    let (left, width) = match index {
        0 => (row.left(), widths[0]),
        1 => (row.left() + widths[0], widths[1]),
        _ => (
            row.left() + widths[0] + widths[1],
            (row.width() - widths[0] - widths[1]).max(0.0),
        ),
    };
    Rect::from_min_size(pos2(left, row.top()), vec2(width, row.height()))
        .shrink2(vec2(CELL_PADDING_X, 0.0))
}

fn cells<'a>(
    texts: &Texts,
    result: &'a ContentResult,
    source: Source,
    position: usize,
) -> Option<Cells<'a>> {
    match source {
        Source::Records => {
            let hit = result.records().get(position)?;
            Some(Cells {
                first: display_text(&hit.record_path).into_owned(),
                second: display_text(&hit.field).into_owned(),
                excerpt: &hit.excerpt,
            })
        }
        Source::Localization => {
            let hit = result.localization().get(position)?;
            Some(Cells {
                first: language_name(texts, hit.language),
                second: display_text(&hit.key).into_owned(),
                excerpt: &hit.excerpt,
            })
        }
        Source::Entries => {
            let hit = result.entries().get(position)?;
            Some(Cells {
                first: display_text(&hit.path).into_owned(),
                second: format::number(hit.line, texts.language()),
                excerpt: &hit.excerpt,
            })
        }
    }
}

type Hit<'a> = (&'a Texts, &'a ContentResult, Source);

/// Returns the laid-out row, laying it out first when it comes into view or becomes selected.
fn laid<'a>(
    ui: &Ui,
    (texts, result, source): Hit<'_>,
    cache: &'a mut Cache,
    row: &Row,
) -> Option<&'a Laid> {
    let width = ui.available_width();
    if cache.row_width.to_bits() != width.to_bits() {
        cache.row_width = width;
        cache.rows.fill_with(|| None);
    }
    let row_rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(width, ROW_HEIGHT));
    let widths: [f32; 3] =
        std::array::from_fn(|index| column_rect(row_rect, row.widths, index).width());
    let slot = cache.rows.get_mut(row.position)?;
    if slot.is_none() {
        let cells = cells(texts, result, source, row.position)?;
        *slot = Some(Laid {
            cells: lay_cells(ui, &cells, widths, Weight::Regular),
            bold: None,
            excerpt: excerpt_galley(ui, cells.excerpt, widths[2]),
        });
    }
    let laid = slot.as_mut()?;
    if row.active && laid.bold.is_none() {
        let cells = cells(texts, result, source, row.position)?;
        laid.bold = Some(lay_cells(ui, &cells, widths, Weight::Bold));
    }
    Some(&*laid)
}

fn lay_cells(ui: &Ui, cells: &Cells<'_>, widths: [f32; 3], weight: Weight) -> [Arc<Galley>; 2] {
    let lay = |text: &str, width| line(ui, plain(text, ROW_FONT, weight), width);
    [lay(&cells.first, widths[0]), lay(&cells.second, widths[1])]
}

fn show_row(ui: &mut Ui, (texts, result, source): Hit<'_>, laid: &Laid, row: &Row) -> (bool, bool) {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let state = RowState::of(row.active, false, row.hovered);
    paint_table_row(ui.painter(), rect, state, row.position % 2 == 1);
    paint_divider(ui.painter(), rect);
    let (_, colour) = state.colours();
    let shown = laid
        .bold
        .as_ref()
        .filter(|_| row.active)
        .unwrap_or(&laid.cells);
    for (index, galley) in shown.iter().enumerate() {
        paint_left(
            ui,
            column_rect(rect, row.widths, index),
            Arc::clone(galley),
            colour,
        );
    }
    let column = column_rect(rect, row.widths, 2);
    let painter = ui
        .painter()
        .with_clip_rect(column.intersect(ui.clip_rect()));
    painter.galley(
        pos2(
            column.left(),
            column.center().y - laid.excerpt.size().y / 2.0,
        ),
        Arc::clone(&laid.excerpt),
        colour,
    );
    response.widget_info(|| {
        let name = cells(texts, result, source, row.position).map_or_else(String::new, |name| {
            texts.format(
                ContentSearchText::Row,
                &[
                    ("place", format!("{} {}", name.first, name.second).into()),
                    ("text", name.excerpt.text().to_owned().into()),
                ],
            )
        });
        WidgetInfo::selected(Role::Row, true, row.active, name)
    });
    focus_inside(ui.painter(), rect, &response);
    (response.clicked(), response.hovered())
}

/// Lays the excerpt out on one line with the match in bold white, cut at `width`; the rest takes
/// the colour the row is painted in.
fn excerpt_galley(ui: &Ui, excerpt: &Excerpt, width: f32) -> Arc<Galley> {
    let plain_format = |weight, colour| TextFormat::simple(font(ROW_FONT, weight), colour);
    let mut job = LayoutJob::default();
    let parts: [(&str, bool); 3] = [
        (excerpt.before(), false),
        (excerpt.matched_text(), true),
        (excerpt.after(), false),
    ];
    for (text, matched) in parts {
        if text.is_empty() {
            continue;
        }
        let format = if matched {
            plain_format(Weight::Bold, palette::WHITE)
        } else {
            plain_format(Weight::Regular, Color32::PLACEHOLDER)
        };
        job.append(text, 0.0, format);
    }
    job.wrap = egui::text::TextWrapping::truncate_at_width(width);
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}
