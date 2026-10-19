use egui::{EventFilter, Id, Key, Rect, Response, Sense, Ui, WidgetInfo, pos2, vec2};

use crate::theme::desktop::{
    FOCUS_OFFSET, PROGRESS_HEIGHT, SLIDER_HEIGHT, SLIDER_HIT_HEIGHT, TRANSITION,
};
use crate::theme::palette;

use super::focus::focus_ring;
use super::indicator::track;

/// How far the keys move a slider: the arrow keys by `step`, Page Up and Page Down by `page`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Steps {
    pub(crate) step: f32,
    pub(crate) page: f32,
}

/// Shows an `input[type="range"]` in the accent orange at `fraction`, named `label` with the
/// accessible value `value`, `width` wide inside a [`SLIDER_HIT_HEIGHT`] row: the track of the
/// progress bar filled up to a round thumb. A click or drag sets it, the arrow keys and Page
/// Up and Page Down move it by `steps`, Home and End go to the ends. Returns its response and
/// the fraction chosen in this frame.
pub(crate) fn slider(
    ui: &mut Ui,
    (label, value): (&str, &str),
    fraction: f32,
    steps: Steps,
    width: f32,
) -> (Response, Option<f32>) {
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, SLIDER_HIT_HEIGHT), Sense::click_and_drag());
    let range = Rect::from_center_size(rect.center(), vec2(width, SLIDER_HEIGHT));
    let travel = range.shrink2(vec2(SLIDER_HEIGHT / 2.0, 0.0));
    let mut chosen = response
        .interact_pointer_pos()
        .filter(|_| response.is_pointer_button_down_on() || response.clicked())
        .map(|pointer| ((pointer.x - travel.left()) / travel.width().max(1.0)).clamp(0.0, 1.0));
    if response.has_focus() {
        chosen = chosen.or_else(|| keyed(ui, response.id, fraction, steps));
    }
    let shown = chosen.unwrap_or(fraction).clamp(0.0, 1.0);
    response.widget_info(|| {
        let mut info = WidgetInfo::slider(true, f64::from(shown), label);
        info.current_text_value = Some(value.to_owned());
        info
    });
    if !ui.is_rect_visible(rect) {
        return (response, chosen);
    }
    let bar = Rect::from_center_size(range.center(), vec2(width, PROGRESS_HEIGHT));
    let thumb_x = travel.width().mul_add(shown, travel.left());
    track(
        ui,
        bar,
        (thumb_x - bar.left()) / bar.width().max(1.0),
        palette::PRIMARY,
    );
    let hover = ui.ctx().animate_bool_with_time(
        response.id,
        response.hovered() && !response.dragged(),
        TRANSITION,
    );
    let thumb = Rect::from_center_size(
        pos2(thumb_x, range.center().y),
        vec2(SLIDER_HEIGHT, SLIDER_HEIGHT),
    );
    let colour = palette::PRIMARY.lerp_to_gamma(palette::ACCENT_LIGHT, hover);
    ui.painter()
        .circle_filled(thumb.center(), SLIDER_HEIGHT / 2.0, colour);
    if response.has_focus() {
        focus_ring(ui, range, palette::PRIMARY, FOCUS_OFFSET);
    }
    (response, chosen)
}

fn keyed(ui: &Ui, id: Id, fraction: f32, steps: Steps) -> Option<f32> {
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
        let presses = |key| f32::from(u8::try_from(input.num_presses(key)).unwrap_or(u8::MAX));
        let pages = (presses(Key::PageUp) - presses(Key::PageDown)) * steps.page;
        let moved = (presses(Key::ArrowRight) - presses(Key::ArrowLeft)).mul_add(steps.step, pages);
        (moved != 0.0).then(|| (fraction + moved).clamp(0.0, 1.0))
    })
}
