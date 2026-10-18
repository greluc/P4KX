use std::collections::HashMap;
use std::sync::Arc;

use egui::{
    Color32, CornerRadius, Galley, Id, Painter, Rect, Role, ScrollArea, Sense, Stroke, Ui,
    WidgetInfo, pos2, vec2,
};
use fluent_bundle::FluentValue;
use p4kx_comparison::LineKind;

use crate::theme::desktop::{
    ACTIVE_BAR, DIFF_ADDED_FILL, DIFF_FONT, DIFF_GUTTER_PADDING_X, DIFF_MARKER_GAP,
    DIFF_REMOVED_FILL, DIFF_ROW_HEIGHT,
};
use crate::theme::palette;
use crate::theme::tokens::{Weight, font};
use crate::{ComparisonSide, DiffCell, DiffDocument, DiffHunk, DiffRow, DiffText, Texts};

const MAX_ROWS: usize = 512;
const MAX_LINE_CHARS: usize = 4096;
const TAB: &str = "    ";
const SIDES: [ComparisonSide; 2] = [ComparisonSide::Old, ComparisonSide::New];

/// The laid-out texts of one row: per side the line number and the line, or the hunk range.
#[derive(Clone)]
struct RowGalleys([Option<(Arc<Galley>, Arc<Galley>)>; 2]);

/// The laid-out texts of the rows shown recently, kept across frames until the diff, the
/// language or the scale changes.
#[derive(Clone, Default)]
struct RowCache {
    key: Option<Id>,
    gutter: f32,
    marker: f32,
    widest: f32,
    markers: Option<[Arc<Galley>; 2]>,
    rows: HashMap<u32, RowGalleys>,
}

/// What the rows need besides the document: the texts, the diff's generation and the hunk
/// navigated to.
pub(super) struct Rows<'a> {
    pub(super) texts: &'a Texts,
    pub(super) document: &'a DiffDocument,
    pub(super) generation: u64,
    pub(super) current: Option<usize>,
    pub(super) scroll_to: Option<u32>,
}

/// Shows the virtual side-by-side rows of `rows.document`; only the visible rows are laid out,
/// once.
pub(super) fn show(ui: &mut Ui, rows: &Rows<'_>) {
    let mut cache = RowCache::load(ui, rows);
    let total = rows.document.rows().len();
    let mut area = ScrollArea::both()
        .id_salt(("comparison-diff-rows", rows.generation))
        .auto_shrink(false);
    if let Some(row) = rows.scroll_to {
        area = area.vertical_scroll_offset(row_top(row as usize));
    }
    area.show_viewport(ui, |ui, viewport| {
        let origin = ui.max_rect().min;
        let half = (viewport.width() / 2.0).floor();
        let text_left = cache.gutter + cache.marker + DIFF_MARKER_GAP;
        let extra = (cache.widest - (half - text_left)).max(0.0);
        ui.allocate_rect(
            Rect::from_min_size(origin, vec2(viewport.width() + extra, row_top(total))),
            Sense::hover(),
        );
        let left = origin.x + viewport.min.x;
        let first = row_at(viewport.min.y.floor());
        let last = row_at(viewport.max.y + DIFF_ROW_HEIGHT - 1.0).min(total);
        let frame = Frame {
            left,
            half,
            text_left,
            scroll: viewport.min.x,
        };
        for index in first..last {
            let top = origin.y + row_top(index);
            paint_row(ui, rows, &mut cache, &frame, (index, top));
        }
        let x = left + half;
        ui.painter().vline(
            x,
            (origin.y + viewport.min.y)..=(origin.y + viewport.max.y),
            Stroke::new(1.0, palette::GRAY_3),
        );
    });
    cache.store(ui);
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a row count fits the precision of a scroll offset"
)]
fn row_top(index: usize) -> f32 {
    index as f32 * DIFF_ROW_HEIGHT
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a non-negative row position saturates into a row index"
)]
fn row_at(y: f32) -> usize {
    (y / DIFF_ROW_HEIGHT).floor().max(0.0) as usize
}

struct Frame {
    left: f32,
    half: f32,
    text_left: f32,
    scroll: f32,
}

impl Frame {
    fn column(&self, side: usize, top: f32) -> Rect {
        let left = if side == 0 {
            self.left
        } else {
            self.left + self.half
        };
        Rect::from_min_size(pos2(left, top), vec2(self.half, DIFF_ROW_HEIGHT))
    }
}

fn paint_row(
    ui: &Ui,
    rows: &Rows<'_>,
    cache: &mut RowCache,
    frame: &Frame,
    (index, top): (usize, f32),
) {
    let Some(&row) = rows.document.rows().get(index) else {
        return;
    };
    let id = u32::try_from(index).unwrap_or(u32::MAX);
    let galleys = cache.row(ui, rows, id, row);
    let painter = ui.painter();
    match row {
        DiffRow::Hunk(hunk) => {
            let rect = frame.column(0, top).union(frame.column(1, top));
            painter.rect_filled(rect, CornerRadius::ZERO, palette::SURFACE_INPUT);
            let current = rows.current == Some(hunk as usize);
            if current {
                let bar = rect.with_max_x(rect.left() + ACTIVE_BAR);
                painter.rect_filled(bar, CornerRadius::ZERO, palette::PRIMARY);
            }
            let colour = if current {
                palette::WHITE
            } else {
                palette::GRAY_2_TEXT
            };
            for (side, galley) in galleys.0.iter().enumerate() {
                if let Some((range, _)) = galley {
                    let cell = frame.column(side, top);
                    let at = pos2(cell.left() + DIFF_GUTTER_PADDING_X, centre(cell, range));
                    painter
                        .with_clip_rect(cell)
                        .galley(at, Arc::clone(range), colour);
                }
            }
            label(ui, rect, (id, 2), || {
                let names: Vec<String> = SIDES
                    .iter()
                    .map(|&side| hunk_text(rows, hunk, side))
                    .collect();
                names.join(" | ")
            });
        }
        DiffRow::Lines { old, new } => {
            for (side, cell) in [old, new].into_iter().enumerate() {
                let (Some(cell), Some(Some((number, text)))) = (cell, galleys.0.get(side)) else {
                    continue;
                };
                let rect = frame.column(side, top);
                paint_cell(
                    &painter.with_clip_rect(rect),
                    cache,
                    frame,
                    cell,
                    (rect, number, text),
                );
                let which = SIDES.get(side).copied().unwrap_or(ComparisonSide::Old);
                label(ui, rect, (id, side), || line_name(rows, which, cell));
            }
        }
    }
}

fn paint_cell(
    painter: &Painter,
    cache: &RowCache,
    frame: &Frame,
    cell: DiffCell,
    (rect, number, text): (Rect, &Arc<Galley>, &Arc<Galley>),
) {
    let (fill, tint, marker) = match cell.kind {
        LineKind::Context => (None, palette::GRAY_2_TEXT, None),
        LineKind::Removed => (Some(DIFF_REMOVED_FILL), palette::DANGER_TEXT, Some(0)),
        LineKind::Added => (Some(DIFF_ADDED_FILL), palette::SUCCESS_TEXT, Some(1)),
    };
    if let Some(fill) = fill {
        painter.rect_filled(rect, CornerRadius::ZERO, fill);
    }
    let number_right = rect.left() + cache.gutter - DIFF_GUTTER_PADDING_X;
    painter.galley(
        pos2(number_right - number.size().x, centre(rect, number)),
        Arc::clone(number),
        tint,
    );
    if let (Some(marker), Some(markers)) = (marker, &cache.markers)
        && let Some(galley) = markers.get(marker)
    {
        let at = pos2(rect.left() + cache.gutter, centre(rect, galley));
        painter.galley(at, Arc::clone(galley), tint);
    }
    let text_rect = rect.with_min_x(rect.left() + frame.text_left);
    let at = pos2(text_rect.left() - frame.scroll, centre(rect, text));
    painter
        .with_clip_rect(text_rect.intersect(painter.clip_rect()))
        .galley(at, Arc::clone(text), palette::GRAY_1);
}

fn centre(rect: Rect, galley: &Galley) -> f32 {
    rect.center().y - galley.size().y / 2.0
}

fn label(ui: &Ui, rect: Rect, (row, part): (u32, usize), name: impl Fn() -> String) {
    let response = ui.interact(
        rect,
        ui.scope_id().with(("diff-row", row, part)),
        Sense::hover(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, name()));
}

fn line_name(rows: &Rows<'_>, side: ComparisonSide, cell: DiffCell) -> String {
    let text = match cell.kind {
        LineKind::Context => DiffText::LineContext,
        LineKind::Removed => DiffText::LineRemoved,
        LineKind::Added => DiffText::LineAdded,
    };
    rows.texts.format(
        text,
        &[
            (
                "line",
                FluentValue::from(cell.line.saturating_add(1).to_string()),
            ),
            ("text", display(rows.document.line(side, cell.line)).into()),
        ],
    )
}

fn hunk_text(rows: &Rows<'_>, hunk: u32, side: ComparisonSide) -> String {
    let Some(DiffHunk { old, new, .. }) = rows.document.hunks().get(hunk as usize) else {
        return String::new();
    };
    let lines = if side == ComparisonSide::Old {
        old
    } else {
        new
    };
    if lines.is_empty() {
        return rows.texts.get(DiffText::HunkEmpty);
    }
    rows.texts.format(
        DiffText::Hunk,
        &[
            ("first", lines.start.saturating_add(1).to_string().into()),
            ("last", lines.end.to_string().into()),
        ],
    )
}

/// Returns `bytes` as one line for display: without its terminator, tabs as spaces and cut after
/// [`MAX_LINE_CHARS`] characters.
fn display(bytes: &[u8]) -> String {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    let text = String::from_utf8_lossy(bytes);
    let mut shown = String::with_capacity(text.len().min(MAX_LINE_CHARS));
    for (count, character) in text.chars().enumerate() {
        if count == MAX_LINE_CHARS {
            shown.push('…');
            break;
        }
        if character == '\t' {
            shown.push_str(TAB);
        } else {
            shown.push(character);
        }
    }
    shown
}

fn layout(ui: &Ui, text: String) -> Arc<Galley> {
    ui.fonts_mut(|fonts| {
        fonts.layout_no_wrap(text, font(DIFF_FONT, Weight::Regular), Color32::PLACEHOLDER)
    })
}

impl RowCache {
    fn load(ui: &Ui, rows: &Rows<'_>) -> Self {
        let key = Id::unique((
            rows.texts.language(),
            rows.generation,
            ui.ctx().pixels_per_point().to_bits(),
        ));
        let mut cache: Self =
            ui.data_mut(|data| std::mem::take(data.get_temp_mut_or_default::<Self>(cache_id())));
        if cache.key != Some(key) {
            let digits = rows.document.line_count().max(1).ilog10().saturating_add(1);
            let digit = layout(ui, "0".to_owned()).size().x;
            let markers = [DiffText::MarkerRemoved, DiffText::MarkerAdded]
                .map(|text| layout(ui, rows.texts.get(text)));
            let (side, line) = rows.document.widest();
            #[expect(
                clippy::cast_precision_loss,
                reason = "a digit count fits an f32 exactly"
            )]
            let numbers = digits as f32 * digit;
            cache = Self {
                key: Some(key),
                gutter: DIFF_GUTTER_PADDING_X.mul_add(2.0, numbers),
                marker: markers.iter().map(|m| m.size().x).fold(0.0, f32::max),
                widest: layout(ui, display(rows.document.line(side, line))).size().x,
                markers: Some(markers),
                rows: HashMap::new(),
            };
        }
        cache
    }

    fn store(self, ui: &Ui) {
        ui.data_mut(|data| *data.get_temp_mut_or_default::<Self>(cache_id()) = self);
    }

    fn row(&mut self, ui: &Ui, rows: &Rows<'_>, id: u32, row: DiffRow) -> RowGalleys {
        if let Some(cached) = self.rows.get(&id) {
            return cached.clone();
        }
        if self.rows.len() >= MAX_ROWS {
            self.rows.clear();
        }
        let galleys = match row {
            DiffRow::Hunk(hunk) => RowGalleys(SIDES.map(|side| {
                let range = layout(ui, hunk_text(rows, hunk, side));
                Some((Arc::clone(&range), range))
            })),
            DiffRow::Lines { old, new } => {
                let side = |side: ComparisonSide, cell: Option<DiffCell>| {
                    cell.map(|cell| {
                        let number = layout(ui, cell.line.saturating_add(1).to_string());
                        let text = layout(ui, display(rows.document.line(side, cell.line)));
                        (number, text)
                    })
                };
                RowGalleys([
                    side(ComparisonSide::Old, old),
                    side(ComparisonSide::New, new),
                ])
            }
        };
        self.rows.insert(id, galleys.clone());
        galleys
    }
}

fn cache_id() -> Id {
    Id::unique("comparison-diff-row-cache")
}

#[cfg(test)]
mod tests {
    use super::display;

    #[test]
    fn a_line_is_shown_without_its_terminator_and_with_tabs_as_spaces() {
        assert_eq!(display(b"\ta=1\r\n"), "    a=1", "terminator and tab");
        assert_eq!(display(b"\xFFx"), "\u{FFFD}x", "invalid UTF-8 replaced");
    }

    #[test]
    fn a_very_long_line_is_cut_with_an_ellipsis() {
        let shown = display(&[b'x'; 5000]);

        assert_eq!(shown.chars().count(), 4097, "cut after 4096 characters");
        assert!(shown.ends_with('…'), "ellipsis");
    }
}
