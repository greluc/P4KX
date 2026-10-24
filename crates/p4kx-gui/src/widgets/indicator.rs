// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::f32::consts::TAU;

use egui::{
    Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui,
    UiBuilder, pos2, vec2,
};

use crate::theme::desktop::{
    BRAND_GAP, BRAND_HEIGHT, BRAND_MARK, PROGRESS_HEIGHT, ROW_HEIGHT, SPINNER_INLINE,
    SPINNER_PERIOD, SPINNER_RING, SPINNER_TRACK, WORDMARK_SIZE,
};
use crate::theme::tokens::{SPACE_2, TRACKING_DISPLAY, Weight};
use crate::theme::{logo, palette};

use super::text::{caps, line, paint_left};

const ARC_SEGMENTS: u8 = 12;
const ARC_SPAN: f32 = TAU / 4.0;

/// Shows a `.spinner` of `size`: a faint orange ring with an orange quarter turning.
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

/// Returns the part of the note cell `rect` of a tree row that is left for the text, after the
/// inline spinner while `spinning`.
pub(crate) fn note_text_rect(rect: Rect, spinning: bool) -> Rect {
    if spinning {
        rect.with_min_x(rect.left() + SPINNER_INLINE + SPACE_2)
    } else {
        rect
    }
}

/// Shows the inline spinner at the start of the note cell `rect` of a tree row, centred in the
/// row.
pub(crate) fn row_spinner(ui: &mut Ui, rect: Rect) {
    let spot = Rect::from_min_size(rect.min, vec2(SPINNER_INLINE, rect.height()));
    ui.scope_builder(
        UiBuilder::new().max_rect(spot.shrink2(vec2(0.0, (ROW_HEIGHT - SPINNER_INLINE) / 2.0))),
        |ui| spinner(ui, SPINNER_INLINE),
    );
}

/// Shows the `.attendance-meter` progress bar at `fraction`, grey while `cancelling`.
pub(crate) fn progress(ui: &mut Ui, fraction: f32, cancelling: bool, width: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, PROGRESS_HEIGHT), Sense::hover());
    let fill = if cancelling {
        palette::GRAY_2
    } else {
        palette::PRIMARY
    };
    track(ui, rect, fraction, fill);
}

/// Paints the `.attendance-meter` track over `rect`, filled with `fill` up to `fraction`.
pub(crate) fn track(ui: &Ui, rect: Rect, fraction: f32, fill: Color32) {
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

/// Shows the logo mark and the upper-case `name` in orange after it.
pub(crate) fn brand(ui: &mut Ui, name: &str) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = BRAND_GAP;
        logo_mark(ui);
        let galley = line(
            ui,
            caps(name, WORDMARK_SIZE, Weight::Bold, TRACKING_DISPLAY),
            f32::INFINITY,
        );
        let (rect, response) =
            ui.allocate_exact_size(vec2(galley.size().x, BRAND_HEIGHT), Sense::hover());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, name));
        paint_left(ui, rect, galley, palette::PRIMARY);
    });
}

fn logo_mark(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(BRAND_MARK, BRAND_MARK), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let context = ui.ctx();
    let pixels = (BRAND_MARK * context.pixels_per_point()).round().max(1.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the size is a small positive whole number"
    )]
    let size = pixels as u32;
    let id = Id::unique("p4kx-logo-mark").with(size);
    let texture = context
        .data(|data| data.get_temp::<TextureHandle>(id))
        .unwrap_or_else(|| {
            let texture = context.load_texture(
                "logo-mark",
                logo::image(logo::Variant::Mark, size),
                TextureOptions::LINEAR,
            );
            context.data_mut(|data| data.insert_temp(id, texture.clone()));
            texture
        });
    ui.painter().image(
        texture.id(),
        rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );
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
