use std::collections::HashMap;
use std::sync::Arc;

use egui::{
    Color32, EventFilter, Galley, Id, Key, Modifiers, Rect, Role, ScrollArea, Sense, Ui, Vec2,
    WidgetInfo, pos2, vec2,
};

use crate::shown_line::display;
use crate::theme::desktop::{XML_FONT, XML_GUTTER_PADDING_X, XML_ROW_HEIGHT};
use crate::theme::palette;
use crate::theme::tokens::{Weight, font};
use crate::widgets::rows::focus_inside;
use crate::{Texts, XmlDocument, XmlText};

const MAX_ROWS: usize = 512;

/// The laid-out line numbers and lines shown recently, kept across frames until the document
/// or the scale changes, and where the view stood in the last frame.
#[derive(Clone, Default)]
struct RowCache {
    key: Option<Id>,
    gutter: f32,
    widest: f32,
    rows: HashMap<u32, (Arc<Galley>, Arc<Galley>)>,
    offset: Vec2,
    view: Vec2,
    content: Vec2,
}

/// Shows the virtual, numbered lines of `document`; only the visible lines are laid out, once.
///
/// The view takes the keyboard focus with Tab and then scrolls with the arrow keys, Page Up,
/// Page Down, Home and End.
pub(super) fn show(ui: &mut Ui, texts: &Texts, document: &XmlDocument) {
    let cache_id = ui.scope_id().with("xml-view-row-cache");
    let (mut cache, fresh) = RowCache::load(ui, cache_id, document);
    let region = ui.available_rect_before_wrap();
    let focus_id = cache_id.with("focus");
    let focus = ui.interact(region, focus_id, Sense::focusable_noninteractive());
    focus.widget_info(|| WidgetInfo::labeled(Role::ScrollView, true, texts.get(XmlText::View)));
    let mut area = ScrollArea::both().id_salt(cache_id).auto_shrink(false);
    if fresh {
        area = area.scroll_offset(Vec2::ZERO);
    }
    if focus.has_focus() {
        let filter = EventFilter {
            horizontal_arrows: true,
            vertical_arrows: true,
            ..EventFilter::default()
        };
        ui.memory_mut(|memory| memory.set_focus_lock_filter(focus_id, filter));
        if let Some(offset) = keyboard_offset(ui, &cache) {
            area = area.scroll_offset(offset);
        }
    }
    let total = document.line_count();
    let output = area.show_viewport(ui, |ui, viewport| {
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
            let rect = Rect::from_min_size(pos2(left, top), vec2(viewport.width(), XML_ROW_HEIGHT));
            paint_row(ui, document, &mut cache, (index, rect), viewport.min.x);
        }
    });
    focus_inside(ui.painter(), output.inner_rect, &focus);
    cache.offset = output.state.clamped_offset();
    cache.view = output.inner_rect.size();
    cache.content = output.content_size;
    ui.data_mut(|data| data.insert_temp(cache_id, cache));
}

/// Returns where a key pressed on the focused view scrolls to, or `None` when none was.
fn keyboard_offset(ui: &Ui, cache: &RowCache) -> Option<Vec2> {
    let page = (cache.view.y - XML_ROW_HEIGHT).max(XML_ROW_HEIGHT);
    let max = (cache.content - cache.view).max(Vec2::ZERO);
    let keys = [
        Key::ArrowDown,
        Key::ArrowUp,
        Key::ArrowRight,
        Key::ArrowLeft,
        Key::PageDown,
        Key::PageUp,
        Key::Home,
        Key::End,
    ];
    let mut offset = None;
    ui.input_mut(|input| {
        for key in keys {
            if input.consume_key(Modifiers::NONE, key) {
                let at = offset.unwrap_or(cache.offset);
                offset = Some(match key {
                    Key::ArrowDown => at + vec2(0.0, XML_ROW_HEIGHT),
                    Key::ArrowUp => at - vec2(0.0, XML_ROW_HEIGHT),
                    Key::ArrowRight => at + vec2(XML_ROW_HEIGHT, 0.0),
                    Key::ArrowLeft => at - vec2(XML_ROW_HEIGHT, 0.0),
                    Key::PageDown => at + vec2(0.0, page),
                    Key::PageUp => at - vec2(0.0, page),
                    Key::Home => vec2(at.x, 0.0),
                    _ => vec2(at.x, max.y),
                });
            }
        }
    });
    offset.map(|offset| offset.clamp(Vec2::ZERO, max))
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
    let response = ui.interact(rect, ui.scope_id().with(("xml-line", id)), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, text.text()));
    painter
        .with_clip_rect(text_rect.intersect(painter.clip_rect()))
        .galley(at, text, palette::GRAY_1);
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
    /// Returns the cache of `document` and whether it was laid out anew.
    fn load(ui: &Ui, cache_id: Id, document: &XmlDocument) -> (Self, bool) {
        let key = Id::unique((
            "xml-view",
            document.serial(),
            ui.ctx().pixels_per_point().to_bits(),
        ));
        let mut cache: Self =
            ui.data_mut(|data| std::mem::take(data.get_temp_mut_or_default::<Self>(cache_id)));
        let fresh = cache.key != Some(key);
        if fresh {
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
                ..Self::default()
            };
        }
        (cache, fresh)
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
