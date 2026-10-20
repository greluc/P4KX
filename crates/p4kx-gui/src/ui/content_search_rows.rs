use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Color32, CornerRadius, Galley, Rect, Role, ScrollArea, Sense, Ui, WidgetInfo, pos2, vec2,
};
use p4kx_search::{Excerpt, Source, display_text};

use crate::texts::language_name;
use crate::theme::desktop::{
    CELL_PADDING_X, ROW_FONT, ROW_HEIGHT, TABLE_HEAD_SIZE, TABLE_HEAD_TRACKING,
};
use crate::theme::palette;
use crate::theme::tokens::{Weight, font};
use crate::widgets::rows::{RowState, focus_inside, paint_divider, paint_table_row, row_weight};
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

/// Shows the hits of the listed source as rows of three columns and returns the position of the
/// hit that was clicked.
pub(super) fn show(
    ui: &mut Ui,
    texts: &Texts,
    result: &ContentResult,
    tab: &ContentSearchTab,
) -> Option<usize> {
    let source = tab.shown();
    let heads = heads(texts, source);
    let widths = widths(ui.available_width());
    head(ui, &heads, widths);
    let total = result.len(source);
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
                for position in range {
                    let Some(cells) = cells(texts, result, source, position) else {
                        continue;
                    };
                    let row = Row {
                        position,
                        active: tab.selected() == Some(position),
                        hovered: hovered == Some(position),
                        widths,
                    };
                    let (was_clicked, is_hovered) = show_row(ui, texts, &cells, &row);
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

struct Row {
    position: usize,
    active: bool,
    hovered: bool,
    widths: [f32; 2],
}

fn heads(texts: &Texts, source: Source) -> [String; 3] {
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
    [
        texts.get(first),
        texts.get(second),
        texts.get(ContentSearchText::ColumnText),
    ]
}

fn widths(available: f32) -> [f32; 2] {
    [
        (available * 0.3).clamp(MIN_FIRST, 420.0),
        (available * 0.18).clamp(MIN_SECOND, 300.0),
    ]
}

fn head(ui: &mut Ui, heads: &[String; 3], widths: [f32; 2]) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette::SURFACE_INPUT);
    let rule = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), rect.max);
    ui.painter()
        .rect_filled(rule, CornerRadius::ZERO, palette::PRIMARY);
    for (index, name) in heads.iter().enumerate() {
        let column = column_rect(rect, widths, index);
        let galley = line(
            ui,
            caps(name, TABLE_HEAD_SIZE, Weight::Bold, TABLE_HEAD_TRACKING),
            column.width(),
        );
        let response = ui.interact(
            column,
            ui.scope_id().with(("content-search-head", index)),
            Sense::hover(),
        );
        response.widget_info(|| WidgetInfo::labeled(Role::ColumnHeader, true, name));
        paint_left(ui, column, galley, palette::GRAY_1);
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

fn show_row(ui: &mut Ui, texts: &Texts, cells: &Cells<'_>, row: &Row) -> (bool, bool) {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let state = RowState::of(row.active, false, row.hovered);
    paint_table_row(ui.painter(), rect, state, row.position % 2 == 1);
    paint_divider(ui.painter(), rect);
    let (_, colour) = state.colours();
    let weight = row_weight(row.active);
    for (index, text) in [&cells.first, &cells.second].into_iter().enumerate() {
        let column = column_rect(rect, row.widths, index);
        let galley = line(ui, plain(text, ROW_FONT, weight), column.width());
        paint_left(ui, column, galley, colour);
    }
    let column = column_rect(rect, row.widths, 2);
    let galley = excerpt_galley(ui, cells.excerpt, column.width(), colour);
    let painter = ui
        .painter()
        .with_clip_rect(column.intersect(ui.clip_rect()));
    painter.galley(
        pos2(column.left(), column.center().y - galley.size().y / 2.0),
        galley,
        colour,
    );
    let name = texts.format(
        ContentSearchText::Row,
        &[
            ("place", format!("{} {}", cells.first, cells.second).into()),
            ("text", cells.excerpt.text().to_owned().into()),
        ],
    );
    response.widget_info(|| WidgetInfo::selected(Role::Row, true, row.active, name.clone()));
    focus_inside(ui.painter(), rect, &response);
    (response.clicked(), response.hovered())
}

/// Lays the excerpt out on one line with the match in bold white, cut at `width`.
fn excerpt_galley(ui: &Ui, excerpt: &Excerpt, width: f32, colour: Color32) -> Arc<Galley> {
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
            plain_format(Weight::Regular, colour)
        };
        job.append(text, 0.0, format);
    }
    job.wrap = egui::text::TextWrapping::truncate_at_width(width);
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}
