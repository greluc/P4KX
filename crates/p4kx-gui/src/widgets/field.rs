use egui::epaint::RectShape;
use egui::layers::ShapeIdx;
use egui::{Rect, Shape, Stroke, StrokeKind, Ui};

use crate::theme::palette;
use crate::theme::tokens::{CORNER_SM, GLOW_PRIMARY};

/// Reserves the place behind a text field where its fill and border are painted once the field
/// has its rectangle.
pub(crate) fn frame_slot(ui: &Ui) -> ShapeIdx {
    ui.painter().add(Shape::Noop)
}

/// Reserves the place behind a text field where its bloom is painted while it has focus.
pub(crate) fn bloom_slot(ui: &Ui) -> ShapeIdx {
    ui.painter().add(Shape::Noop)
}

/// Fills `slot` with the `input[type="text"]` frame at `rect`: the control fill and a
/// `control-border` border that turns `color-primary` when `highlighted`.
pub(crate) fn paint_frame(ui: &Ui, slot: ShapeIdx, rect: Rect, highlighted: bool) {
    let border = if highlighted {
        palette::PRIMARY
    } else {
        palette::CONTROL_BORDER
    };
    ui.painter().set(
        slot,
        RectShape::new(
            rect,
            CORNER_SM,
            palette::CONTROL_BG,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        ),
    );
}

/// Fills `slot` with the `glow-primary` bloom around `rect`.
pub(crate) fn paint_bloom(ui: &Ui, slot: ShapeIdx, rect: Rect) {
    ui.painter()
        .set(slot, GLOW_PRIMARY.as_shape(rect, CORNER_SM));
}
