// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;
use std::time::Duration;

use egui::{Align, Galley, Id, Layout, Role, ScrollArea, Sense, Ui, UiBuilder, WidgetInfo, vec2};
use fluent_bundle::FluentValue;

use super::audio_controls::clock;
use super::model_view;
use super::search::eyebrow;
use crate::theme::desktop::{AUDIO_CONTROL_GAP, ROW_HEIGHT, SLIDER_HIT_HEIGHT};
use crate::theme::model_metrics::{
    MODEL_ANIMATIONS_WIDTH, MODEL_BAR_HEIGHT, MODEL_BONE_STROKE, MODEL_OVERLAY_GAP,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, Weight};
use crate::viewer::Look;
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::slider::{Steps, slider};
use crate::widgets::text::{line, paint_left, plain};
use crate::widgets::toggle::{checkbox, radio};
use crate::{Camera, Control, Language, ModelText, Playback, Scene, Texts};

const TENTH: Duration = Duration::from_millis(100);
const SEEK_STEP: f32 = 1.0;
const SEEK_PAGE: f32 = 5.0;

/// What the 3D view and the controls of the Model tab ask for in one frame.
#[derive(Debug, Default)]
pub(super) struct ViewerAsk {
    pub(super) camera: Option<Camera>,
    pub(super) control: Option<Control>,
}

/// Shows the 3D view of `scene` seen by `camera` with the skeleton over it when `playback` asks
/// for it, under it the bar with the playback controls, and beside it the list of animations the
/// model has.
pub(super) fn view(
    ui: &mut Ui,
    texts: &Texts,
    path: &str,
    (scene, camera): (&Arc<Scene>, &Camera),
    playback: Option<&Playback>,
) -> ViewerAsk {
    let Some(playback) = playback else {
        let camera = model_view::viewport(ui, texts, path, (scene, camera), (Look::default(), 0.0));
        return ViewerAsk {
            camera,
            control: None,
        };
    };
    let look = Look {
        pose: Some(Arc::clone(playback.pose())),
        bones: playback
            .shows_skeleton()
            .then(|| MODEL_BONE_STROKE * ui.ctx().pixels_per_point()),
    };
    let listed = !playback.animations().is_empty();
    let total = ui.available_size();
    let side = if listed {
        MODEL_ANIMATIONS_WIDTH + MODEL_OVERLAY_GAP
    } else {
        0.0
    };
    let column = Layout::top_down(Align::Min);
    let mut ask = ViewerAsk::default();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing = vec2(MODEL_OVERLAY_GAP, 0.0);
        let width = (total.x - side).max(1.0);
        ui.allocate_ui_with_layout(vec2(width, total.y), column, |ui| {
            let reserve = MODEL_OVERLAY_GAP + MODEL_BAR_HEIGHT;
            let below = ui
                .max_rect()
                .with_min_y(ui.max_rect().bottom() - MODEL_BAR_HEIGHT);
            ask.camera = model_view::viewport(ui, texts, path, (scene, camera), (look, reserve));
            ask.control = ui
                .scope_builder(
                    UiBuilder::new()
                        .max_rect(below)
                        .layout(Layout::left_to_right(Align::Center)),
                    |ui| bar(ui, texts, playback),
                )
                .inner;
        });
        if listed {
            ui.allocate_ui_with_layout(vec2(MODEL_ANIMATIONS_WIDTH, total.y), column, |ui| {
                ask.control = list(ui, texts, playback).or(ask.control);
            });
        }
    });
    ask
}

#[derive(Clone)]
struct Labels {
    play: Arc<str>,
    pause: Arc<str>,
    position: Arc<str>,
    looping: Arc<str>,
    skeleton: Arc<str>,
    animations: Arc<str>,
    bind_pose: Arc<str>,
}

fn labels(ui: &Ui, texts: &Texts) -> Labels {
    let id = Id::unique("p4kx-model-playback-labels");
    let language = texts.language();
    if let Some((cached, labels)) = ui.data(|data| data.get_temp::<(Language, Labels)>(id))
        && cached == language
    {
        return labels;
    }
    let get = |text: ModelText| Arc::<str>::from(texts.get(text));
    let labels = Labels {
        play: get(ModelText::Play),
        pause: get(ModelText::Pause),
        position: get(ModelText::Position),
        looping: get(ModelText::Loop),
        skeleton: get(ModelText::Skeleton),
        animations: get(ModelText::Animations),
        bind_pose: get(ModelText::BindPose),
    };
    ui.data_mut(|data| data.insert_temp(id, (language, labels.clone())));
    labels
}

/// Shows the bar under the view with the controls of `playback`: the skeleton checkbox and, when
/// the model has animations, play or pause, the time, the position slider and the loop checkbox.
pub(super) fn bar(ui: &mut Ui, texts: &Texts, playback: &Playback) -> Option<Control> {
    let labels = labels(ui, texts);
    let timing = (!playback.animations().is_empty()).then(|| time(ui, texts, playback));
    let mut control = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = AUDIO_CONTROL_GAP;
        if let Some((time, width)) = &timing {
            let toggle = if playback.is_playing() {
                &labels.pause
            } else {
                &labels.play
            };
            let enabled = playback.selected().is_some();
            if button::button(ui, Kind::Outline, Size::Compact, toggle, enabled).clicked() {
                control = Some(Control::Toggle);
            }
            let (rect, response) =
                ui.allocate_exact_size(vec2(*width, SLIDER_HIT_HEIGHT), Sense::hover());
            response.widget_info(|| WidgetInfo::labeled(Role::Label, true, time.text()));
            paint_left(ui, rect, Arc::clone(time), palette::GRAY_1);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let mut shown = playback.shows_skeleton();
            if checkbox(ui, &mut shown, &labels.skeleton).changed() {
                control = Some(Control::Skeleton(shown));
            }
            if let Some((time, _)) = &timing {
                let mut looping = playback.is_looping();
                if checkbox(ui, &mut looping, &labels.looping).changed() {
                    control = Some(Control::Loop(looping));
                }
                let named = (labels.position.as_ref(), time.text());
                let steps = steps(playback.duration());
                let width = ui.available_width();
                let (_, chosen) = slider(ui, named, playback.fraction(), steps, width);
                if let Some(fraction) = chosen {
                    control = Some(Control::Seek(fraction));
                }
            }
        });
    });
    control
}

fn steps(duration: f32) -> Steps {
    let fraction = |seconds: f32| {
        if duration > 0.0 {
            (seconds / duration).min(1.0)
        } else {
            1.0
        }
    };
    Steps {
        step: fraction(SEEK_STEP),
        page: fraction(SEEK_PAGE),
    }
}

fn tenths(seconds: f32) -> Duration {
    let tenths = (seconds.max(0.0) * 10.0).floor();
    TENTH.mul_f32(tenths)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TimeKey {
    language: Language,
    scale: u32,
    selected: Option<usize>,
    tenths: u64,
}

fn time(ui: &Ui, texts: &Texts, playback: &Playback) -> (Arc<Galley>, f32) {
    let id = Id::unique("p4kx-model-playback-time");
    let position = tenths(playback.time());
    let key = TimeKey {
        language: texts.language(),
        scale: ui.ctx().pixels_per_point().to_bits(),
        selected: playback.selected(),
        tenths: u64::try_from(position.as_millis() / TENTH.as_millis()).unwrap_or(0),
    };
    let cached = ui.data(|data| data.get_temp::<(TimeKey, Arc<Galley>, f32)>(id));
    if let Some((cached, galley, width)) = &cached
        && *cached == key
    {
        return (Arc::clone(galley), *width);
    }
    let lay = |position: &str, duration: &str| {
        let args = [
            ("position", FluentValue::from(position.to_owned())),
            ("duration", FluentValue::from(duration.to_owned())),
        ];
        let text = texts.format(ModelText::Time, &args);
        line(ui, plain(text, FS_XS, Weight::Regular), f32::INFINITY)
    };
    let duration = clock(tenths(playback.duration()), true);
    let galley = lay(&clock(position, true), &duration);
    let width = match cached {
        Some((before, _, width))
            if before.selected == key.selected
                && before.language == key.language
                && before.scale == key.scale =>
        {
            width
        }
        _ => ('0'..='9')
            .map(|digit| {
                let widest: String = duration
                    .chars()
                    .map(|c| if c.is_ascii_digit() { digit } else { c })
                    .collect();
                lay(&widest, &widest).size().x
            })
            .fold(galley.size().x, f32::max),
    };
    ui.data_mut(|data| data.insert_temp(id, (key, Arc::clone(&galley), width)));
    (galley, width)
}

fn list(ui: &mut Ui, texts: &Texts, playback: &Playback) -> Option<Control> {
    let labels = labels(ui, texts);
    let mut control = None;
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), SLIDER_HIT_HEIGHT * 0.5),
        Sense::hover(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::Heading, true, labels.animations.as_ref()));
    let heading = line(ui, eyebrow(&labels.animations), rect.width());
    paint_left(ui, rect, heading, palette::GRAY_2_TEXT);
    let rows = playback.animations().len() + 1;
    ScrollArea::vertical()
        .id_salt("p4kx-model-animations")
        .auto_shrink([false, false])
        .show_rows(ui, ROW_HEIGHT, rows, |ui, range| {
            for row in range {
                let animation = row.checked_sub(1);
                let name = animation
                    .map_or_else(|| Some(labels.bind_pose.as_ref()), |at| playback.name(at));
                let selected = playback.selected() == animation;
                if radio(ui, selected, name.unwrap_or_default(), true).clicked() {
                    control = Some(Control::Select(animation));
                }
            }
        });
    control
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{steps, tenths};

    #[test]
    fn the_time_shown_is_cut_to_a_tenth_of_a_second() {
        assert_eq!(
            tenths(1.299),
            Duration::from_millis(1200),
            "cut, not rounded"
        );
        assert_eq!(tenths(-1.0), Duration::ZERO, "never before the start");
    }

    #[test]
    fn the_keys_move_the_position_by_a_second_and_a_page_by_five() {
        let ten = steps(10.0);

        assert!((ten.step - 0.1).abs() < 1.0e-6, "a second of ten");
        assert!((ten.page - 0.5).abs() < 1.0e-6, "five seconds of ten");
        assert!(
            (steps(0.5).step - 1.0).abs() < 1.0e-6,
            "a short animation takes the whole"
        );
        assert!((steps(0.0).page - 1.0).abs() < 1.0e-6, "no length");
    }
}
