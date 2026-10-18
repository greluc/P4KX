use std::collections::HashMap;
use std::sync::Arc;

use egui::{Color32, Galley, Id, Rect, Role, ScrollArea, Sense, Ui, WidgetInfo, pos2, vec2};

use super::diff_line::display;
use crate::XmlDocument;
use crate::theme::desktop::{XML_FONT, XML_GUTTER_PADDING_X, XML_ROW_HEIGHT};
use crate::theme::palette;
use crate::theme::tokens::{Weight, font};

const MAX_ROWS: usize = 512;

/// The laid-out line numbers and lines shown recently, kept across frames until the document
/// or the scale changes.
#[derive(Clone, Default)]
struct RowCache {
    key: Option<Id>,
    gutter: f32,
    widest: f32,
    rows: HashMap<u32, (Arc<Galley>, Arc<Galley>)>,
}

/// Shows the virtual, numbered lines of `document`, identified by `serial` for the galleys it
/// keeps; only the visible lines are laid out, once.
pub(super) fn show(ui: &mut Ui, document: &XmlDocument, serial: u64) {
    let mut cache = RowCache::load(ui, document, serial);
    let total = document.line_count();
    ScrollArea::both()
        .id_salt(("xml-view-rows", serial))
        .auto_shrink(false)
        .show_viewport(ui, |ui, viewport| {
            let origin = ui.max_rect().min;
            let width = viewport.width().max(cache.gutter + cache.widest);
            ui.allocate_rect(
                Rect::from_min_size(origin, vec2(width, row_top(total))),
                Sense::hover(),
            );
            let left = origin.x + viewport.min.x;
            let first = row_at(viewport.min.y.floor());
            let last = row_at(viewport.max.y + XML_ROW_HEIGHT - 1.0).min(total);
            for index in first..last {
                let top = origin.y + row_top(index);
                let rect =
                    Rect::from_min_size(pos2(left, top), vec2(viewport.width(), XML_ROW_HEIGHT));
                paint_row(ui, document, &mut cache, (index, rect), viewport.min.x);
            }
        });
    cache.store(ui);
}

fn paint_row(
    ui: &Ui,
    document: &XmlDocument,
    cache: &mut RowCache,
    (index, rect): (usize, Rect),
    scroll: f32,
) {
    let id = u32::try_from(index).unwrap_or(u32::MAX);
    let (number, text) = cache.row(ui, document, id);
    let painter = ui.painter_at(rect);
    let number_right = rect.left() + cache.gutter - XML_GUTTER_PADDING_X;
    painter.galley(
        pos2(number_right - number.size().x, centre(rect, &number)),
        number,
        palette::GRAY_2_TEXT,
    );
    let text_rect = rect.with_min_x(rect.left() + cache.gutter);
    let at = pos2(text_rect.left() - scroll, centre(rect, &text));
    painter
        .with_clip_rect(text_rect.intersect(painter.clip_rect()))
        .galley(at, text, palette::GRAY_1);
    let response = ui.interact(rect, ui.scope_id().with(("xml-line", id)), Sense::hover());
    response
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, display(document.line(index)).text));
}

fn centre(rect: Rect, galley: &Galley) -> f32 {
    rect.center().y - galley.size().y / 2.0
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a line count fits the precision of a scroll offset"
)]
fn row_top(index: usize) -> f32 {
    index as f32 * XML_ROW_HEIGHT
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a non-negative row position saturates into a line index"
)]
fn row_at(y: f32) -> usize {
    (y / XML_ROW_HEIGHT).floor().max(0.0) as usize
}

fn layout(ui: &Ui, text: String) -> Arc<Galley> {
    ui.fonts_mut(|fonts| {
        fonts.layout_no_wrap(text, font(XML_FONT, Weight::Regular), Color32::PLACEHOLDER)
    })
}

fn layout_line(ui: &Ui, bytes: &[u8]) -> Arc<Galley> {
    let job = display(bytes).job(&font(XML_FONT, Weight::Regular));
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

impl RowCache {
    fn load(ui: &Ui, document: &XmlDocument, serial: u64) -> Self {
        let key = Id::unique(("xml-view", serial, ui.ctx().pixels_per_point().to_bits()));
        let mut cache: Self =
            ui.data_mut(|data| std::mem::take(data.get_temp_mut_or_default::<Self>(cache_id())));
        if cache.key != Some(key) {
            let digits = document.line_count().max(1).ilog10().saturating_add(1);
            let digit = layout(ui, "0".to_owned()).size().x;
            #[expect(
                clippy::cast_precision_loss,
                reason = "a digit count fits an f32 exactly"
            )]
            let numbers = digits as f32 * digit;
            cache = Self {
                key: Some(key),
                gutter: XML_GUTTER_PADDING_X.mul_add(2.0, numbers),
                widest: layout_line(ui, document.line(document.widest())).size().x,
                rows: HashMap::new(),
            };
        }
        cache
    }

    fn store(self, ui: &Ui) {
        ui.data_mut(|data| *data.get_temp_mut_or_default::<Self>(cache_id()) = self);
    }

    fn row(&mut self, ui: &Ui, document: &XmlDocument, id: u32) -> (Arc<Galley>, Arc<Galley>) {
        if let Some((number, text)) = self.rows.get(&id) {
            return (Arc::clone(number), Arc::clone(text));
        }
        if self.rows.len() >= MAX_ROWS {
            self.rows.clear();
        }
        let number = layout(ui, id.saturating_add(1).to_string());
        let text = layout_line(ui, document.line(id as usize));
        self.rows
            .insert(id, (Arc::clone(&number), Arc::clone(&text)));
        (number, text)
    }
}

fn cache_id() -> Id {
    Id::unique("xml-view-row-cache")
}
