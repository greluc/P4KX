// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Color32, EventFilter, Id, Key, Rect, Response, Sense, Ui, WidgetInfo, pos2, vec2};

use crate::theme::desktop::{
    DISABLED_OPACITY, FOCUS_OFFSET, PROGRESS_HEIGHT, SLIDER_HEIGHT, SLIDER_HIT_HEIGHT, TRANSITION,
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
    named: (&str, &str),
    fraction: f32,
    steps: Steps,
    width: f32,
) -> (Response, Option<f32>) {
    slider_if(
        ui,
        named,
        fraction,
        steps,
        Shape {
            width,
            enabled: true,
        },
    )
}

/// How wide a slider is and whether it reacts.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Shape {
    pub(crate) width: f32,
    pub(crate) enabled: bool,
}

/// Shows a slider like [`slider`], dimmed and deaf to the pointer and the keys when `shape` is
/// not enabled.
pub(crate) fn slider_if(
    ui: &mut Ui,
    (label, value): (&str, &str),
    fraction: f32,
    steps: Steps,
    Shape { width, enabled }: Shape,
) -> (Response, Option<f32>) {
    let sense = if enabled {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, SLIDER_HIT_HEIGHT), sense);
    let range = Rect::from_center_size(rect.center(), vec2(width, SLIDER_HEIGHT));
    let travel = range.shrink2(vec2(SLIDER_HEIGHT / 2.0, 0.0));
    let mut chosen = response
        .interact_pointer_pos()
        .filter(|_| enabled && (response.is_pointer_button_down_on() || response.clicked()))
        .map(|pointer| ((pointer.x - travel.left()) / travel.width().max(1.0)).clamp(0.0, 1.0));
    if enabled && response.has_focus() {
        chosen = chosen.or_else(|| keyed(ui, response.id, fraction, steps));
    }
    let shown = chosen.unwrap_or(fraction).clamp(0.0, 1.0);
    response.widget_info(|| {
        let mut info = WidgetInfo::slider(enabled, f64::from(shown), label);
        info.current_text_value = Some(value.to_owned());
        info
    });
    if !ui.is_rect_visible(rect) {
        return (response, chosen);
    }
    let dim = |colour: Color32| {
        if enabled {
            colour
        } else {
            colour.gamma_multiply(DISABLED_OPACITY)
        }
    };
    let bar = Rect::from_center_size(range.center(), vec2(width, PROGRESS_HEIGHT));
    let thumb_x = travel.width().mul_add(shown, travel.left());
    track(
        ui,
        bar,
        (thumb_x - bar.left()) / bar.width().max(1.0),
        dim(palette::PRIMARY),
    );
    let hover = ui.ctx().animate_bool_with_time(
        response.id,
        enabled && response.hovered() && !response.dragged(),
        TRANSITION,
    );
    let thumb = Rect::from_center_size(
        pos2(thumb_x, range.center().y),
        vec2(SLIDER_HEIGHT, SLIDER_HEIGHT),
    );
    let colour = dim(palette::PRIMARY.lerp_to_gamma(palette::ACCENT_LIGHT, hover));
    ui.painter()
        .circle_filled(thumb.center(), SLIDER_HEIGHT / 2.0, colour);
    if enabled && response.has_focus() {
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
