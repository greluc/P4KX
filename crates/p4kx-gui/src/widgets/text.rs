use std::sync::Arc;

use egui::{
    Align2, Color32, CursorIcon, Galley, OpenUrl, Rect, Response, RichText, Role, Sense, Stroke,
    TextStyle, TextWrapMode, Ui, WidgetInfo, WidgetText, pos2,
};

use crate::theme::desktop::{FOCUS_OFFSET, FOCUS_WIDTH};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, Weight, font};

/// Returns `text` in upper case at `size` in `weight`, tracked by `tracking` em.
pub(crate) fn caps(text: &str, size: f32, weight: Weight, tracking: f32) -> RichText {
    RichText::new(text.to_uppercase())
        .font(font(size, weight))
        .extra_letter_spacing(size * tracking)
}

/// Returns `text` as it is, at `size` in `weight`: data keeps its case and gets no tracking.
pub(crate) fn plain(text: impl Into<String>, size: f32, weight: Weight) -> RichText {
    RichText::new(text).font(font(size, weight))
}

/// Lays `text` out on one line, cut with an ellipsis at `width`.
pub(crate) fn line(ui: &Ui, text: RichText, width: f32) -> Arc<Galley> {
    WidgetText::from(text).into_galley(ui, Some(TextWrapMode::Truncate), width, TextStyle::Body)
}

/// Paints `galley` left-aligned and vertically centred in `rect` in `color`.
pub(crate) fn paint_left(ui: &Ui, rect: Rect, galley: Arc<Galley>, color: Color32) {
    let at = Align2::LEFT_CENTER
        .align_size_within_rect(galley.size(), rect)
        .min;
    ui.painter().galley(at, galley, color);
}

/// Paints `galley` right-aligned and vertically centred in `rect` in `color`.
pub(crate) fn paint_right(ui: &Ui, rect: Rect, galley: Arc<Galley>, color: Color32) {
    let at = Align2::RIGHT_CENTER
        .align_size_within_rect(galley.size(), rect)
        .min;
    ui.painter().galley(at, galley, color);
}

/// Shows a link to `url` in the design system's link colours and opens it on a click.
pub(crate) fn link(ui: &mut Ui, text: &str, url: &str) -> Response {
    let galley = line(ui, plain(text, FS_XS, Weight::Bold), f32::INFINITY);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Link, true, text));
    let hovered = response.hovered();
    let color = if hovered {
        palette::ACCENT_LIGHT
    } else {
        palette::PRIMARY
    };
    ui.painter().galley(rect.min, galley, color);
    if hovered {
        let y = rect.bottom();
        ui.painter().hline(
            rect.x_range(),
            y,
            Stroke::new(FOCUS_WIDTH / 2.0, palette::ACCENT_LIGHT),
        );
    }
    if response.has_focus() {
        super::focus::focus_ring_square(ui, rect, palette::PRIMARY, FOCUS_OFFSET);
    }
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    if response.clicked() {
        ui.ctx().open_url(OpenUrl::new_tab(url));
    }
    response
}

/// Paints a hairline under `rect`.
pub(crate) fn hairline_below(ui: &Ui, rect: Rect) {
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, palette::GRAY_3),
    );
}

/// Returns the top-left position at which a `size` square is centred in `rect`.
pub(crate) fn centred(rect: Rect, size: f32) -> egui::Pos2 {
    pos2(rect.center().x - size / 2.0, rect.center().y - size / 2.0)
}
