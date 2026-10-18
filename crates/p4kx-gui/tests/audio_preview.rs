//! The Audio tab of the preview over a synthetic Archive with WEMs, driven headless through a
//! fake audio output.

mod sounding;

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{AudioStatus, Language, Model, PreviewTab, SoundStatus};
use sounding::{
    FakeOutput, TestResult, audio, opened, player, select, tone_sample, vorbis, wait_until,
};

fn press(harness: &mut egui_kittest::Harness<'_, Model>, label: &str) {
    harness.get_by_label(label).click();
    for _ in 0..3 {
        harness.step();
    }
}

fn near(left: f32, right: f32) -> bool {
    (left - right).abs() < 1e-4
}

#[test]
fn selecting_a_wem_shows_codec_channels_sample_rate_and_duration() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, &FakeOutput::default())?;

    select(&mut harness, "tone.wem")?;

    assert_eq!(
        harness.state().preview_tab(),
        PreviewTab::Audio,
        "Audio tab"
    );
    assert!(
        harness
            .query_by_label("Codec PCM Channels 2 Sample rate 48,000 Hz Duration 0:01.000")
            .is_some(),
        "facts"
    );
    assert!(harness.query_by_label("0:00 / 0:01").is_some(), "time");
    assert!(
        harness
            .query_by_role_and_label(Role::Slider, "Position")
            .is_some(),
        "position bar"
    );
    Ok(())
}

#[test]
fn play_starts_the_output_and_pause_silences_it() -> TestResult {
    let output = FakeOutput::default();
    let (_folder, mut harness) = opened(Language::BritishEnglish, &output)?;
    select(&mut harness, "tone.wem")?;

    press(&mut harness, "Play");
    let played = output.pull(8).ok_or("no output started")?;

    assert_eq!(output.started(), 1, "one stream");
    assert!(
        harness.query_by_label("Pause").is_some(),
        "the button pauses now"
    );
    let expected: Vec<f32> = (0..4)
        .flat_map(|frame| [tone_sample(frame, 0), tone_sample(frame, 1)])
        .collect();
    assert!(
        played
            .iter()
            .zip(&expected)
            .all(|(&got, &want)| near(got, want)),
        "the tone plays: {played:?}"
    );

    press(&mut harness, "Pause");
    let paused = output.pull(8).ok_or("the stream ended")?;

    assert!(paused.iter().all(|&sample| sample == 0.0), "silence");
    assert!(
        harness.query_by_label("Play").is_some(),
        "the button plays again"
    );
    Ok(())
}

#[test]
fn clicking_the_position_bar_seeks() -> TestResult {
    let output = FakeOutput::default();
    let (_folder, mut harness) = opened(Language::BritishEnglish, &output)?;
    select(&mut harness, "tone.wem")?;
    press(&mut harness, "Play");

    harness
        .get_by_role_and_label(Role::Slider, "Position")
        .click();
    harness.step();
    harness.step();
    let position = player(harness.state())
        .map(|player| {
            player.pcm().sample_rate() as usize * player.position().as_millis() as usize / 1000
        })
        .ok_or("no player")?;
    let played = output.pull(2).ok_or("no output started")?;

    assert!(
        (23_500..24_500).contains(&position),
        "about the middle: frame {position}"
    );
    let frame = (position - 50..position + 50).find(|&frame| {
        near(played[0], tone_sample(frame, 0)) && near(played[1], tone_sample(frame, 1))
    });
    assert!(frame.is_some(), "plays from there: {played:?}");
    Ok(())
}

#[test]
fn the_volume_bar_scales_the_samples() -> TestResult {
    let output = FakeOutput::default();
    let (_folder, mut harness) = opened(Language::BritishEnglish, &output)?;
    select(&mut harness, "tone.wem")?;
    press(&mut harness, "Play");
    let _start = output.pull(8).ok_or("no output started")?;

    harness
        .get_by_role_and_label(Role::Slider, "Volume")
        .click();
    harness.step();
    harness.step();
    let volume = harness.state().audio_volume();
    let half = output.pull(8).ok_or("the stream ended")?;

    assert!((volume - 0.5).abs() < 0.02, "about half: {volume}");
    let expected: Vec<f32> = (4..8)
        .flat_map(|frame| [tone_sample(frame, 0), tone_sample(frame, 1)])
        .map(|sample| sample * volume)
        .collect();
    assert!(
        half.iter()
            .zip(&expected)
            .all(|(&got, &want)| near(got, want)),
        "scaled by the volume: {half:?}"
    );
    Ok(())
}

#[test]
fn the_keyboard_moves_the_volume_bar() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, &FakeOutput::default())?;
    select(&mut harness, "tone.wem")?;

    harness
        .get_by_role_and_label(Role::Slider, "Volume")
        .focus();
    harness.step();
    harness.key_press(egui::Key::Home);
    harness.step();
    let silent = harness.state().audio_volume();
    harness.key_press(egui::Key::ArrowRight);
    harness.step();
    let raised = harness.state().audio_volume();

    assert!(near(silent, 0.0), "Home mutes: {silent}");
    assert!(
        (raised - 0.1).abs() < 1e-3,
        "an arrow raises by a tenth: {raised}"
    );
    Ok(())
}

#[test]
fn changing_the_selection_stops_the_sound() -> TestResult {
    let output = FakeOutput::default();
    let (_folder, mut harness) = opened(Language::BritishEnglish, &output)?;
    select(&mut harness, "tone.wem")?;
    press(&mut harness, "Play");

    harness.get_by_label("readme.txt").click();
    wait_until(&mut harness, |model| audio(model).is_none())?;

    assert!(output.pull(2).is_none(), "the stream ended");
    assert_eq!(harness.state().preview_tab(), PreviewTab::Text, "Text tab");
    assert!(harness.query_by_label("Audio").is_none(), "no Audio tab");
    Ok(())
}

#[test]
fn ima_adpcm_decodes_and_plays() -> TestResult {
    let output = FakeOutput::default();
    let (_folder, mut harness) = opened(Language::BritishEnglish, &output)?;

    select(&mut harness, "ima.WEM")?;
    press(&mut harness, "Play");

    assert!(
        harness
            .query_by_label_contains("Codec IMA ADPCM Channels 1")
            .is_some(),
        "facts"
    );
    assert_eq!(output.started(), 1, "plays");
    Ok(())
}

#[test]
fn wwise_vorbis_decodes_and_plays() -> TestResult {
    let output = FakeOutput::default();
    let (_folder, mut harness) = opened(Language::BritishEnglish, &output)?;

    select(&mut harness, "engine.wem")?;
    press(&mut harness, "Play");
    let played = output.pull(4096).ok_or("no output started")?;

    assert!(
        harness
            .query_by_label_contains("Codec Wwise Vorbis Channels 2 Sample rate 48,000 Hz")
            .is_some(),
        "facts"
    );
    assert_eq!(
        player(harness.state()).map(|player| player.pcm().frames()),
        Some(u64::from(vorbis().decoded_samples())),
        "every frame decoded"
    );
    assert!(played.iter().any(|&sample| sample != 0.0), "the tone plays");
    Ok(())
}

#[test]
fn a_codec_without_a_decoder_is_named() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, &FakeOutput::default())?;

    select(&mut harness, "voice.wem")?;

    assert!(
        harness
            .query_by_label(
                "The preview cannot play Opus sounds. The Text and Hex tabs still show the \
                 data of Data\\Sounds\\voice.wem."
            )
            .is_some(),
        "message"
    );
    assert!(harness.query_by_label("Play").is_none(), "no controls");
    Ok(())
}

#[test]
fn a_sound_too_long_to_hold_is_not_decoded() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, &FakeOutput::default())?;

    select(&mut harness, "endless.wem")?;

    assert!(
        harness
            .query_by_label_contains("Data\\Sounds\\endless.wem is too long to preview")
            .is_some(),
        "message"
    );
    Ok(())
}

#[test]
fn a_corrupt_wem_names_the_reason() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, &FakeOutput::default())?;

    select(&mut harness, "broken.wem")?;

    assert!(
        matches!(
            audio(harness.state()).map(p4kx_gui::AudioPreview::status),
            Some(AudioStatus::Failed(Some(_)))
        ),
        "failed"
    );
    assert!(
        harness
            .query_by_label_contains("Cannot decode Data\\Sounds\\broken.wem: ")
            .is_some(),
        "message"
    );
    Ok(())
}

#[test]
fn a_failing_audio_output_shows_a_message_instead_of_playing() -> TestResult {
    let output = FakeOutput::failing("no audio device");
    let (_folder, mut harness) = opened(Language::BritishEnglish, &output)?;
    select(&mut harness, "tone.wem")?;

    press(&mut harness, "Play");

    assert!(
        harness
            .query_by_label("The sound cannot be played: no audio device")
            .is_some(),
        "message"
    );
    assert!(harness.query_by_label("Play").is_some(), "not playing");
    press(&mut harness, "Play");
    assert_eq!(output.started(), 2, "playing again retries the output");
    Ok(())
}

#[test]
fn decoding_runs_off_the_window_thread() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, &FakeOutput::default())?;
    let browser = harness.state_mut().browser_mut().ok_or("not opened")?;
    let id = (0..u32::try_from(browser.index().len())?)
        .find(|&id| browser.index().name(id) == "tone.wem")
        .ok_or("no sound")?;

    browser.select_entry(id, || {});
    let busy = browser.audio_mut().map(|audio| audio.is_busy());
    let ready = browser.audio_mut().is_some_and(|audio| {
        matches!(
            audio.status(),
            AudioStatus::Opened(_, SoundStatus::Ready(_))
        )
    });

    assert_eq!(busy, Some(true), "the WEM is read in the background");
    assert!(!ready, "selecting returns before the sound is decoded");
    Ok(())
}

#[test]
fn the_german_audio_tab_uses_german_texts() -> TestResult {
    let (_folder, mut harness) = opened(Language::German, &FakeOutput::default())?;

    select(&mut harness, "tone.wem")?;

    assert!(harness.query_by_label("Abspielen").is_some(), "play");
    assert!(
        harness
            .query_by_label("Codec PCM Kanäle 2 Abtastrate 48.000 Hz Dauer 0:01.000")
            .is_some(),
        "facts"
    );
    assert!(
        harness
            .query_by_role_and_label(Role::Slider, "Lautstärke")
            .is_some(),
        "volume"
    );
    Ok(())
}
