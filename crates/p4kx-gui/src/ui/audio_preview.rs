use std::sync::Arc;
use std::time::Duration;

use egui::{Galley, Id, Role, Sense, Ui, WidgetInfo, vec2};
use fluent_bundle::FluentValue;

use super::search::eyebrow;
use crate::theme::desktop::{
    AUDIO_CONTROL_GAP, AUDIO_POSITION_MIN_WIDTH, AUDIO_VOLUME_WIDTH, SLIDER_HIT_HEIGHT,
    SPINNER_INLINE,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames;
use crate::widgets::indicator::spinner;
use crate::widgets::slider::slider;
use crate::widgets::summary::{Fact, FactsStrip};
use crate::widgets::text::{line, paint_left, plain};
use crate::{
    AudioPreview, AudioStatus, AudioText, Language, MAX_PREVIEW_PCM, Model, PREVIEW_SIZE_LIMIT,
    Player, SoundInfo, SoundStatus, Text, Texts, format,
};

const POSITION_REFRESH: Duration = Duration::from_millis(100);
const SEEK_STEP: Duration = Duration::from_secs(5);
const VOLUME_STEP: f32 = 0.1;

/// What the user did with the controls of the Audio tab.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum AudioAction {
    Toggle,
    Seek(f32),
    Volume(f32),
}

/// Applies `action` to the playback of the selected WEM.
pub(super) fn apply(model: &mut Model, action: AudioAction) {
    match action {
        AudioAction::Toggle => model.toggle_playback(),
        AudioAction::Seek(fraction) => model.seek_playback(fraction),
        AudioAction::Volume(volume) => model.set_audio_volume(volume),
    }
}

/// Shows the Audio tab of the WEM at `path` of `size` bytes: its facts, the playback controls
/// at `volume` and the messages. Returns what the user did.
pub(super) fn show(
    ui: &mut Ui,
    texts: &Texts,
    (path, size): (&str, u64),
    audio: &AudioPreview,
    volume: f32,
) -> Option<AudioAction> {
    let status = audio.status();
    if let AudioStatus::Opened(info, _) = status {
        facts(ui, texts, audio.serial(), info).show(ui);
    }
    let path_arg = || ("path", FluentValue::from(path.to_owned()));
    let language = texts.language();
    frames::preview_body()
        .show(ui, |ui| match status {
            AudioStatus::Opening => {
                loading(ui, &texts.format(Text::PreviewLoading, &[path_arg()]));
                None
            }
            AudioStatus::TooLarge => {
                let args = [
                    path_arg(),
                    ("size", format::size(size, language).into()),
                    ("limit", format::size(PREVIEW_SIZE_LIMIT, language).into()),
                ];
                ui.label(texts.format(Text::PreviewTooLarge, &args));
                None
            }
            AudioStatus::Failed(reason) => {
                ui.colored_label(palette::DANGER_TEXT, failure(texts, path, reason));
                None
            }
            AudioStatus::ArchiveChanged => {
                ui.colored_label(palette::WARNING, texts.get(Text::PreviewArchiveChanged));
                None
            }
            AudioStatus::Opened(_, sound) => sound_body(ui, texts, path, audio, sound, volume),
        })
        .inner
}

fn sound_body(
    ui: &mut Ui,
    texts: &Texts,
    path: &str,
    audio: &AudioPreview,
    sound: SoundStatus<'_>,
    volume: f32,
) -> Option<AudioAction> {
    let path_arg = || ("path", FluentValue::from(path.to_owned()));
    match sound {
        SoundStatus::Decoding => {
            loading(ui, &texts.format(AudioText::Decoding, &[path_arg()]));
            None
        }
        SoundStatus::Unsupported(codec) => {
            let codec = ("codec", FluentValue::from(codec.to_string()));
            let text = texts.format(AudioText::Unsupported, &[codec, path_arg()]);
            ui.colored_label(palette::DANGER_TEXT, text);
            None
        }
        SoundStatus::TooLong => {
            let limit = format::size(MAX_PREVIEW_PCM, texts.language());
            let args = [path_arg(), ("limit", limit.into())];
            ui.colored_label(
                palette::DANGER_TEXT,
                texts.format(AudioText::TooLong, &args),
            );
            None
        }
        SoundStatus::Failed(reason) => {
            ui.colored_label(palette::DANGER_TEXT, failure(texts, path, reason));
            None
        }
        SoundStatus::Ready(player) => controls(ui, texts, audio.serial(), player, volume),
    }
}

fn controls(
    ui: &mut Ui,
    texts: &Texts,
    serial: u64,
    player: &Player,
    volume: f32,
) -> Option<AudioAction> {
    let labels = labels(ui, texts);
    let time = time(ui, texts, serial, player);
    let mut action = None;
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
        galley(ui, &time, palette::GRAY_1);
        let rest = 2.0f32.mul_add(
            AUDIO_CONTROL_GAP,
            labels.volume.size().x + AUDIO_VOLUME_WIDTH,
        );
        let width = (ui.available_width() - rest).max(AUDIO_POSITION_MIN_WIDTH);
        let step = step(player);
        if let Some(fraction) = slider(ui, &labels.position, player.fraction(), step, width) {
            action = Some(AudioAction::Seek(fraction));
        }
        galley(ui, &labels.volume, palette::GRAY_2_TEXT);
        let name = &labels.volume_name;
        if let Some(chosen) = slider(ui, name, volume, VOLUME_STEP, AUDIO_VOLUME_WIDTH) {
            action = Some(AudioAction::Volume(chosen));
        }
    });
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

fn step(player: &Player) -> f32 {
    let duration = player.duration().as_secs_f32();
    if duration > 0.0 {
        (SEEK_STEP.as_secs_f32() / duration).min(1.0)
    } else {
        1.0
    }
}

fn galley(ui: &mut Ui, galley: &Arc<Galley>, colour: egui::Color32) {
    let (rect, response) =
        ui.allocate_exact_size(vec2(galley.size().x, SLIDER_HIT_HEIGHT), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, galley.text()));
    paint_left(ui, rect, Arc::clone(galley), colour);
}

fn loading(ui: &mut Ui, text: &str) {
    ui.horizontal(|ui| {
        spinner(ui, SPINNER_INLINE);
        ui.label(text);
    });
}

fn failure(texts: &Texts, path: &str, reason: Option<&str>) -> String {
    let path_arg = || ("path", FluentValue::from(path.to_owned()));
    reason.map_or_else(
        || texts.format(Text::PreviewStopped, &[path_arg()]),
        |reason| {
            let reason = ("reason", FluentValue::from(reason.to_owned()));
            texts.format(AudioText::Failed, &[path_arg(), reason])
        },
    )
}

/// Returns `time` as minutes and seconds, with hours once it reaches one and with
/// milliseconds when `precise`.
fn clock(time: Duration, precise: bool) -> String {
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
struct Key {
    serial: u64,
    second: u64,
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
    let key = Key {
        serial: 0,
        second: 0,
        language: texts.language(),
        pixels_per_point: ui.ctx().pixels_per_point(),
    };
    if let Some((cached, labels)) = ui.data(|data| data.get_temp::<(Key, Labels)>(id))
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

/// Returns the position and length of the sound, laid out once per whole second shown.
fn time(ui: &Ui, texts: &Texts, serial: u64, player: &Player) -> Arc<Galley> {
    let id = Id::unique("p4kx-audio-time");
    let position = player.position();
    let key = Key {
        serial,
        second: position.as_secs(),
        language: texts.language(),
        pixels_per_point: ui.ctx().pixels_per_point(),
    };
    if let Some((cached, time)) = ui.data(|data| data.get_temp::<(Key, Arc<Galley>)>(id))
        && cached == key
    {
        return time;
    }
    let args = [
        ("position", FluentValue::from(clock(position, false))),
        (
            "duration",
            FluentValue::from(clock(player.duration(), false)),
        ),
    ];
    let text = texts.format(AudioText::Time, &args);
    let time = line(ui, plain(text, FS_XS, Weight::Regular), f32::INFINITY);
    ui.data_mut(|data| data.insert_temp(id, (key, Arc::clone(&time))));
    time
}
/// Returns the facts of `info`, laid out once per audio preview, language and scale.
fn facts(ui: &Ui, texts: &Texts, serial: u64, info: &SoundInfo) -> Arc<FactsStrip> {
    let id = Id::unique("p4kx-audio-facts");
    let key = Key {
        serial,
        second: 0,
        language: texts.language(),
        pixels_per_point: ui.ctx().pixels_per_point(),
    };
    if let Some((cached, strip)) = ui.data(|data| data.get_temp::<(Key, Arc<FactsStrip>)>(id))
        && cached == key
    {
        return strip;
    }
    let language = texts.language();
    let codec = info.codec.to_string();
    let channels = info.channels.to_string();
    let rate = format::number(u64::from(info.sample_rate), language);
    let rate = texts.format(AudioText::Hertz, &[("rate", rate.into())]);
    let duration = info.duration.map_or_else(
        || texts.get(AudioText::Unknown),
        |duration| clock(duration, true),
    );
    let keys = [
        AudioText::Codec,
        AudioText::Channels,
        AudioText::SampleRate,
        AudioText::Duration,
    ]
    .map(|text| texts.get(text));
    let values = [&codec, &channels, &rate, &duration];
    let laid: Vec<Fact<'_>> = keys
        .iter()
        .zip(values)
        .map(|(key, value)| Fact { key, value })
        .collect();
    let strip = Arc::new(FactsStrip::lay_out(ui, &laid));
    ui.data_mut(|data| data.insert_temp(id, (key, Arc::clone(&strip))));
    strip
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
