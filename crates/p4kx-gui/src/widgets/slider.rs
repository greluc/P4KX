use egui::{CornerRadius, EventFilter, Key, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, vec2};

use crate::theme::desktop::{FOCUS_OFFSET, PROGRESS_HEIGHT, SLIDER_HIT_HEIGHT, TRANSITION};
use crate::theme::palette;

use super::button::focus_ring;

/// Shows the `.attendance-meter` progress bar at `fraction` as a slider named `label`, `width`
/// wide inside a [`SLIDER_HIT_HEIGHT`] row: a click or drag sets it, the arrow keys move it by
/// `step`, Home and End go to the ends. Returns the fraction chosen in this frame.
pub(crate) fn slider(
    ui: &mut Ui,
    label: &str,
    fraction: f32,
    step: f32,
    width: f32,
) -> Option<f32> {
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, SLIDER_HIT_HEIGHT), Sense::click_and_drag());
    let bar = Rect::from_center_size(rect.center(), vec2(width, PROGRESS_HEIGHT));
    let mut chosen = response
        .interact_pointer_pos()
        .filter(|_| response.is_pointer_button_down_on() || response.clicked())
        .map(|pointer| ((pointer.x - bar.left()) / bar.width().max(1.0)).clamp(0.0, 1.0));
    if response.has_focus() {
        chosen = chosen.or_else(|| keyed(ui, response.id, fraction, step));
    }
    let shown = chosen.unwrap_or(fraction).clamp(0.0, 1.0);
    response.widget_info(|| WidgetInfo::slider(true, f64::from(shown), label));
    if !ui.is_rect_visible(rect) {
        return chosen;
    }
    let hover = ui.ctx().animate_bool_with_time(
        response.id,
        response.hovered() || response.dragged(),
        TRANSITION,
    );
    let border = palette::GRAY_3.lerp_to_gamma(palette::PRIMARY, hover);
    let painter = ui.painter();
    painter.rect(
        bar,
        CornerRadius::ZERO,
        palette::SURFACE_INPUT,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    let done = Rect::from_min_size(bar.min, vec2(bar.width() * shown, bar.height()));
    painter.rect_filled(done.shrink(1.0), CornerRadius::ZERO, palette::PRIMARY);
    if response.has_focus() {
        focus_ring(ui, bar, palette::PRIMARY, FOCUS_OFFSET);
    }
    chosen
}

fn keyed(ui: &Ui, id: egui::Id, fraction: f32, step: f32) -> Option<f32> {
    ui.memory_mut(|memory| {
        memory.set_focus_lock_filter(
            id,
            EventFilter {
                horizontal_arrows: true,
                ..Default::default()
            },
        );
    });
    ui.input(|input| {
        if input.key_pressed(Key::Home) {
            return Some(0.0);
        }
        if input.key_pressed(Key::End) {
            return Some(1.0);
        }
        let steps = f32::from(u8::try_from(input.num_presses(Key::ArrowRight)).unwrap_or(u8::MAX))
            - f32::from(u8::try_from(input.num_presses(Key::ArrowLeft)).unwrap_or(u8::MAX));
        (steps != 0.0).then(|| steps.mul_add(step, fraction).clamp(0.0, 1.0))
    })
}
