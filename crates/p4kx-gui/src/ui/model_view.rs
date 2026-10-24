// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use egui::{
    Align, EventFilter, Layout, PointerButton, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, WidgetInfo, pos2, vec2,
};
use fluent_bundle::FluentValue;

use super::viewer_input::{self, Moves};
use crate::theme::desktop::FACTS_PADDING_X;
use crate::theme::model_metrics::{MODEL_HINT_MIN_WIDTH, MODEL_OVERLAY_GAP, MODEL_VIEW_MIN_HEIGHT};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, Weight};
use crate::viewer::{Look, Subject, paint_callback};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::rows::focus_inside;
use crate::widgets::summary::{Fact, FactsStrip};
use crate::widgets::text::{line, paint_right, plain};
use crate::{Camera, Language, ModelText, Scene, Texts, Uniforms, format};

#[derive(Debug, Clone, Copy, PartialEq)]
struct FactsKey {
    serial: u64,
    language: Language,
    pixels_per_point: f32,
}

/// Shows the facts bar of `scene`, with the overview of the controls in the space beside the
/// facts.
pub(super) fn facts(ui: &mut Ui, texts: &Texts, scene: &Scene) {
    let strip = laid_out_facts(ui, texts, scene);
    let bar = strip.show(ui);
    let hint_text = texts.get(ModelText::Hint);
    let area = Rect::from_min_max(
        pos2(bar.left() + strip.width(), bar.top()),
        pos2(bar.right() - FACTS_PADDING_X, bar.bottom()),
    );
    ui.interact(
        area,
        ui.scope_id().with(("p4kx-model-hint", scene.serial())),
        Sense::hover(),
    )
    .widget_info(|| WidgetInfo::labeled(Role::Label, true, hint_text.as_str()));
    if area.width() >= MODEL_HINT_MIN_WIDTH {
        let galley = line(ui, plain(&hint_text, FS_XS, Weight::Regular), area.width());
        paint_right(ui, area, galley, palette::GRAY_2_TEXT);
    }
}

fn laid_out_facts(ui: &Ui, texts: &Texts, scene: &Scene) -> Arc<FactsStrip> {
    let id = ui.scope_id().with("p4kx-model-facts");
    let key = FactsKey {
        serial: scene.serial(),
        language: texts.language(),
        pixels_per_point: ui.ctx().pixels_per_point(),
    };
    if let Some((cached, strip)) = ui.data(|data| data.get_temp::<(FactsKey, Arc<FactsStrip>)>(id))
        && cached == key
    {
        return strip;
    }
    let language = texts.language();
    let count = |value: usize| format::number(u64::try_from(value).unwrap_or(u64::MAX), language);
    let bounds = scene.bounds();
    let side = |axis: usize| {
        let min = bounds.min.get(axis).copied().unwrap_or(0.0);
        let max = bounds.max.get(axis).copied().unwrap_or(0.0);
        format::metres(max - min, language)
    };
    let extent = texts.format(
        ModelText::Dimensions,
        &[
            ("width", side(0).into()),
            ("depth", side(1).into()),
            ("height", side(2).into()),
        ],
    );
    let keys =
        [ModelText::Vertices, ModelText::Triangles, ModelText::Extent].map(|text| texts.get(text));
    let values = [
        count(scene.vertices().len()),
        count(scene.triangle_count()),
        extent,
    ];
    let laid: Vec<Fact<'_>> = keys
        .iter()
        .zip(&values)
        .map(|(key, value)| Fact { key, value })
        .collect();
    let strip = Arc::new(FactsStrip::lay_out(ui, &laid));
    ui.data_mut(|data| data.insert_temp(id, (key, Arc::clone(&strip))));
    strip
}

/// Shows the 3D view of `scene` seen by `camera`, reads the pointer and the keyboard and lays the
/// reset action over it, drawing the model in `look` and leaving `reserve` points of height
/// below it. Returns the camera when it moved.
pub(super) fn viewport(
    ui: &mut Ui,
    texts: &Texts,
    path: &str,
    (scene, camera): (&Arc<Scene>, &Camera),
    (look, reserve): (Look, f32),
) -> Option<Camera> {
    let describe = || {
        let args = [
            ("path", FluentValue::from(path.to_owned())),
            ("vertices", scene.vertices().len().to_string().into()),
            ("triangles", scene.triangle_count().to_string().into()),
        ];
        texts.format(ModelText::View, &args)
    };
    canvas(
        ui,
        (texts, &describe),
        (
            |_: &Uniforms, _: u32| Some(Subject::Model(Arc::clone(scene))),
            camera,
        ),
        (look, reserve),
    )
}

/// Shows the 3D view of what `subject` picks for the view's uniforms and height in device
/// pixels, seen by `camera`, named for assistive technology by `describe`; see [`viewport`].
pub(super) fn canvas(
    ui: &mut Ui,
    (texts, describe): (&Texts, &dyn Fn() -> String),
    (subject, camera): (impl FnOnce(&Uniforms, u32) -> Option<Subject>, &Camera),
    (look, reserve): (Look, f32),
) -> Option<Camera> {
    let size = (ui.available_size() - vec2(0.0, reserve))
        .max(vec2(MODEL_VIEW_MIN_HEIGHT, MODEL_VIEW_MIN_HEIGHT));
    let sense = Sense::click_and_drag().union(Sense::focusable_noninteractive());
    let (rect, response) = ui.allocate_exact_size(size, sense);
    response.widget_info(|| WidgetInfo::labeled(Role::Canvas, true, describe()));
    let aspect = rect.width() / rect.height().max(1.0);
    let mut framed = camera.clone();
    if !framed.is_fitted() {
        framed.fit(aspect);
    }
    let asked = collect(ui, &response);
    viewer_input::apply(&mut framed, &asked, aspect, rect.height());
    let uniforms = framed.uniforms(aspect);
    let pixels = [
        device_pixels(ui, rect.width()),
        device_pixels(ui, rect.height()),
    ];
    if let Some(subject) = subject(&uniforms, pixels[1]) {
        ui.painter()
            .add(paint_callback(rect, subject, uniforms, (pixels, look)));
    }
    ui.painter().rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, palette::GRAY_3),
        StrokeKind::Inside,
    );
    focus_inside(ui.painter(), rect, &response);
    if reset_button(ui, texts, rect, !framed.is_home()) {
        framed.reset(aspect);
        ui.ctx().request_repaint();
    }
    (framed != *camera).then_some(framed)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a view is a few thousand pixels at most and never negative here"
)]
fn device_pixels(ui: &Ui, length: f32) -> u32 {
    (length * ui.ctx().pixels_per_point()).round().max(1.0) as u32
}

fn collect(ui: &Ui, response: &Response) -> Moves {
    let mut moves = Moves::default();
    let (secondary, middle, shift) = ui.input(|input| {
        (
            input.pointer.button_down(PointerButton::Secondary),
            input.pointer.button_down(PointerButton::Middle),
            input.modifiers.shift,
        )
    });
    if response.dragged() {
        let drag = response.drag_delta();
        if secondary || middle || shift {
            moves.pan = drag;
        } else {
            moves.orbit = drag;
        }
    }
    if response.hovered() {
        ui.input(|input| {
            moves.wheel = input.smooth_scroll_delta.y;
            moves.pinch = input.zoom_delta();
        });
    }
    if response.has_focus() {
        let filter = EventFilter {
            horizontal_arrows: true,
            vertical_arrows: true,
            ..EventFilter::default()
        };
        ui.memory_mut(|memory| memory.set_focus_lock_filter(response.id, filter));
        ui.input_mut(|input| viewer_input::read_keys(input, &mut moves));
    }
    moves
}

fn reset_button(ui: &mut Ui, texts: &Texts, rect: Rect, enabled: bool) -> bool {
    let mut clicked = false;
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(rect.shrink(MODEL_OVERLAY_GAP))
            .layout(Layout::right_to_left(Align::Min)),
        |ui| {
            let label = texts.get(ModelText::Reset);
            clicked = button::button(ui, Kind::Ghost, Size::Compact, &label, enabled).clicked();
        },
    );
    clicked
}
