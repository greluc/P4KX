use std::f32::consts::TAU;

use egui::{Color32, CornerRadius, Image, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::theme::desktop::{
    BRAND_GAP, BRAND_MARK, PROGRESS_HEIGHT, SPINNER_PERIOD, SPINNER_RING, SPINNER_TRACK,
    WORDMARK_SIZE,
};
use crate::theme::tokens::{TRACKING_DISPLAY, Weight};
use crate::theme::{logo, palette};

use super::text::{caps, line, paint_left};

const ARC_SEGMENTS: u8 = 12;
const ARC_SPAN: f32 = TAU / 4.0;

/// Shows a `.krt-spinner` of `size`: a faint orange ring with an orange quarter turning.
pub(crate) fn spinner(ui: &mut Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let radius = (size - SPINNER_RING) / 2.0;
    let centre = rect.center();
    let painter = ui.painter();
    painter.circle_stroke(centre, radius, Stroke::new(SPINNER_RING, SPINNER_TRACK));
    let time = ui.input(|input| input.time);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the phase of one turn needs no more than f32 precision"
    )]
    let start = ((time / SPINNER_PERIOD).fract() as f32) * TAU;
    let stroke = Stroke::new(SPINNER_RING, palette::PRIMARY);
    let point = |angle: f32| centre + radius * vec2(angle.cos(), angle.sin());
    for step in 0..ARC_SEGMENTS {
        let from = start + ARC_SPAN * f32::from(step) / f32::from(ARC_SEGMENTS);
        let to = start + ARC_SPAN * f32::from(step + 1) / f32::from(ARC_SEGMENTS);
        painter.line_segment([point(from), point(to)], stroke);
    }
    ui.ctx().request_repaint();
}

/// Shows the `.attendance-meter` progress bar at `fraction`, grey while `cancelling`.
pub(crate) fn progress(ui: &mut Ui, fraction: f32, cancelling: bool, width: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, PROGRESS_HEIGHT), Sense::hover());
    let fill = if cancelling {
        palette::GRAY_2
    } else {
        palette::PRIMARY
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::ZERO,
        palette::SURFACE_INPUT,
        Stroke::new(1.0, palette::GRAY_3),
        StrokeKind::Inside,
    );
    let done = Rect::from_min_size(
        rect.min,
        vec2(rect.width() * fraction.clamp(0.0, 1.0), rect.height()),
    );
    painter.rect_filled(done.shrink(1.0), CornerRadius::ZERO, fill);
}

/// Shows the brand: the glyph and the upper-case `name` in orange.
pub(crate) fn brand(ui: &mut Ui, name: &str) {
    if let Some(mark) = logo::mark(ui.ctx()) {
        ui.add(Image::new(&mark).fit_to_exact_size(vec2(BRAND_MARK, BRAND_MARK)));
        ui.add_space(BRAND_GAP - ui.spacing().item_spacing.x);
    }
    let galley = line(
        ui,
        caps(name, WORDMARK_SIZE, Weight::Bold, TRACKING_DISPLAY),
        f32::INFINITY,
    );
    let (rect, response) =
        ui.allocate_exact_size(vec2(galley.size().x, BRAND_MARK), Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, name));
    paint_left(ui, rect, galley, palette::PRIMARY);
}

/// Paints a warning triangle of `size` with its top-left at `rect.min`.
pub(crate) fn warning_icon(ui: &Ui, rect: Rect, colour: Color32) {
    let stroke = Stroke::new(2.0, colour);
    let at = |x: f32, y: f32| {
        pos2(
            x.mul_add(rect.width(), rect.left()),
            y.mul_add(rect.height(), rect.top()),
        )
    };
    let painter = ui.painter();
    painter.line_segment([at(0.5, 0.1), at(0.95, 0.88)], stroke);
    painter.line_segment([at(0.95, 0.88), at(0.05, 0.88)], stroke);
    painter.line_segment([at(0.05, 0.88), at(0.5, 0.1)], stroke);
    painter.line_segment([at(0.5, 0.38), at(0.5, 0.6)], stroke);
    painter.circle_filled(at(0.5, 0.74), 1.2, colour);
}

/// Paints a search glass of `size` centred in `rect`.
pub(crate) fn search_icon(ui: &Ui, rect: Rect, colour: Color32) {
    let stroke = Stroke::new(2.0, colour);
    let at = |x: f32, y: f32| {
        pos2(
            x.mul_add(rect.width(), rect.left()),
            y.mul_add(rect.height(), rect.top()),
        )
    };
    let painter = ui.painter();
    painter.circle_stroke(at(0.45, 0.45), rect.width() * 0.3, stroke);
    painter.line_segment([at(0.68, 0.68), at(0.9, 0.9)], stroke);
}
