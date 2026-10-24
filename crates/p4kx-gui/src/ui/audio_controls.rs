// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;
use std::time::Duration;

use egui::{Galley, Id, Key, Role, Sense, Ui, WidgetInfo, vec2};
use fluent_bundle::FluentValue;

use super::audio_preview::AudioAction;
use super::search::eyebrow;
use crate::theme::desktop::{AUDIO_CONTROL_GAP, SLIDER_HIT_HEIGHT, SLIDER_LABEL_GAP, SLIDER_WIDTH};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::slider::{Steps, slider};
use crate::widgets::text::{line, paint_left, plain};
use crate::{AudioText, Language, Player, Texts};

const POSITION_REFRESH: Duration = Duration::from_millis(100);
const SEEK_STEP: Duration = Duration::from_secs(5);
const SEEK_PAGE: Duration = Duration::from_secs(30);
const VOLUME: Steps = Steps {
    step: 0.1,
    page: 0.25,
};

/// Shows the playback controls of `player` at `volume`: play or pause, the time, the position
/// and the volume. Space plays or pauses while no other control has the focus. Returns what the
/// user did.
pub(super) fn controls(
    ui: &mut Ui,
    texts: &Texts,
    serial: u64,
    player: &Player,
    volume: f32,
) -> Option<AudioAction> {
    let labels = labels(ui, texts);
    let (time, time_width) = time(ui, texts, serial, player);
    let focused = ui.memory(egui::Memory::focused);
    let mut action = None;
    let mut own = [None; 2];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = AUDIO_CONTROL_GAP;
        let toggle = if player.is_playing() {
            &labels.pause
        } else {
            &labels.play
        };
        if button::button(ui, Kind::Outline, Size::Compact, toggle, true).clicked() {
            action = Some(AudioAction::Toggle);
        }
        galley(ui, &time, time_width, palette::GRAY_1);
        let rest = labels.volume.size().x + SLIDER_LABEL_GAP + SLIDER_WIDTH + AUDIO_CONTROL_GAP;
        let width = (ui.available_width() - rest).max(SLIDER_WIDTH);
        let named = (labels.position.as_ref(), time.text());
        let (seek, chosen) = slider(ui, named, player.fraction(), steps(player), width);
        own[0] = Some(seek.id);
        if let Some(fraction) = chosen {
            action = Some(AudioAction::Seek(fraction));
        }
        ui.spacing_mut().item_spacing.x = SLIDER_LABEL_GAP;
        galley(
            ui,
            &labels.volume,
            labels.volume.size().x,
            palette::GRAY_2_TEXT,
        );
        let named = (labels.volume_name.as_ref(), labels.volume_name.as_ref());
        let (loudness, chosen) = slider(ui, named, volume, VOLUME, SLIDER_WIDTH);
        own[1] = Some(loudness.id);
        if let Some(chosen) = chosen {
            action = Some(AudioAction::Volume(chosen));
        }
    });
    let space = ui.input(|input| input.key_pressed(Key::Space))
        && !ui.ctx().egui_wants_keyboard_input()
        && focused.is_none_or(|id| own.contains(&Some(id)));
    if space {
        action = Some(AudioAction::Toggle);
    }
    if let Some(reason) = player.failure() {
        let reason = ("reason", FluentValue::from(reason.to_owned()));
        let text = texts.format(AudioText::OutputFailed, &[reason]);
        ui.colored_label(palette::DANGER_TEXT, text);
    }
    if player.is_playing() {
        ui.ctx().request_repaint_after(POSITION_REFRESH);
    }
    action
}

fn steps(player: &Player) -> Steps {
    let duration = player.duration().as_secs_f32();
    let fraction = |step: Duration| {
        if duration > 0.0 {
            (step.as_secs_f32() / duration).min(1.0)
        } else {
            1.0
        }
    };
    Steps {
        step: fraction(SEEK_STEP),
        page: fraction(SEEK_PAGE),
    }
}

fn galley(ui: &mut Ui, galley: &Arc<Galley>, width: f32, colour: egui::Color32) {
    let (rect, response) = ui.allocate_exact_size(vec2(width, SLIDER_HIT_HEIGHT), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, galley.text()));
    paint_left(ui, rect, Arc::clone(galley), colour);
}

/// Returns `time` as minutes and seconds, with hours once it reaches one and with
/// milliseconds when `precise`.
pub(super) fn clock(time: Duration, precise: bool) -> String {
    let seconds = time.as_secs();
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    let whole = if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    };
    if precise {
        format!("{whole}.{:03}", time.subsec_millis())
    } else {
        whole
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LabelKey {
    language: Language,
    pixels_per_point: f32,
}

/// The texts of the controls in the current language, the volume label laid out.
#[derive(Clone)]
struct Labels {
    play: Arc<str>,
    pause: Arc<str>,
    position: Arc<str>,
    volume_name: Arc<str>,
    volume: Arc<Galley>,
}

fn labels(ui: &Ui, texts: &Texts) -> Labels {
    let id = Id::unique("p4kx-audio-labels");
    let key = LabelKey {
        language: texts.language(),
        pixels_per_point: ui.ctx().pixels_per_point(),
    };
    if let Some((cached, labels)) = ui.data(|data| data.get_temp::<(LabelKey, Labels)>(id))
        && cached == key
    {
        return labels;
    }
    let volume_name = texts.get(AudioText::Volume);
    let labels = Labels {
        play: texts.get(AudioText::Play).into(),
        pause: texts.get(AudioText::Pause).into(),
        position: texts.get(AudioText::Position).into(),
        volume: line(ui, eyebrow(&volume_name), f32::INFINITY),
        volume_name: volume_name.into(),
    };
    ui.data_mut(|data| data.insert_temp(id, (key, labels.clone())));
    labels
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TimeKey {
    serial: u64,
    second: u64,
    labels: LabelKey,
}

/// Returns the position and length of the sound, laid out once per whole second shown, and the
/// width that the widest time of the sound needs, so the position bar keeps its place.
fn time(ui: &Ui, texts: &Texts, serial: u64, player: &Player) -> (Arc<Galley>, f32) {
    let id = Id::unique("p4kx-audio-time");
    let position = player.position();
    let key = TimeKey {
        serial,
        second: position.as_secs(),
        labels: LabelKey {
            language: texts.language(),
            pixels_per_point: ui.ctx().pixels_per_point(),
        },
    };
    let cached = ui.data(|data| data.get_temp::<(TimeKey, Arc<Galley>, f32)>(id));
    if let Some((cached, time, width)) = &cached
        && *cached == key
    {
        return (Arc::clone(time), *width);
    }
    let lay = |position: &str, duration: &str| {
        let args = [
            ("position", FluentValue::from(position.to_owned())),
            ("duration", FluentValue::from(duration.to_owned())),
        ];
        let text = texts.format(AudioText::Time, &args);
        line(ui, plain(text, FS_XS, Weight::Regular), f32::INFINITY)
    };
    let duration = clock(player.duration(), false);
    let time = lay(&clock(position, false), &duration);
    let width = match cached {
        Some((cached, _, width)) if cached.serial == serial && cached.labels == key.labels => width,
        _ => ('0'..='9')
            .map(|digit| {
                let widest: String = duration
                    .chars()
                    .map(|c| if c.is_ascii_digit() { digit } else { c })
                    .collect();
                lay(&widest, &widest).size().x
            })
            .fold(time.size().x, f32::max),
    };
    ui.data_mut(|data| data.insert_temp(id, (key, Arc::clone(&time), width)));
    (time, width)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::clock;

    #[test]
    fn the_clock_shows_minutes_and_seconds_and_hours_once_reached() {
        assert_eq!(clock(Duration::from_millis(65_432), false), "1:05", "short");
        assert_eq!(
            clock(Duration::from_millis(65_432), true),
            "1:05.432",
            "precise"
        );
        assert_eq!(clock(Duration::from_secs(3_725), false), "1:02:05", "hours");
    }
}
