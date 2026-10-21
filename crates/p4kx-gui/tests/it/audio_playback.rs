//! Playing a WEM in the Audio tab through a fake audio output: play, pause, seek, volume, the
//! keyboard, and what stops or pauses the sound.

use crate::sounding;

use egui::Key;
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model, PreviewTab, WorkTab};
use p4kx_testkit::test_signal;
use sounding::{FakeOutput, TestResult, audio, opened, player, select, wait_until};

fn press(harness: &mut egui_kittest::Harness<'_, Model>, label: &str) {
    harness.get_by_label(label).click();
    for _ in 0..3 {
        harness.step();
    }
}

fn near(left: f32, right: f32) -> bool {
    (left - right).abs() < 1e-4
}

/// Returns the sample of the one-second test tone at `frame` and `channel` as the output gets
/// it.
fn tone_sample(frame: usize, channel: usize) -> f32 {
    test_signal(48_000, 2)
        .get(frame * 2 + channel)
        .map_or(0.0, |&sample| f32::from(sample) / 32_768.0)
}

fn position(model: &Model) -> f32 {
    player(model).map_or(-1.0, p4kx_gui::Player::fraction)
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
fn playing_again_at_the_end_starts_over() -> TestResult {
    let output = FakeOutput::default();
    let (_folder, mut harness) = opened(Language::BritishEnglish, &output)?;
    select(&mut harness, "tone.wem")?;
    press(&mut harness, "Play");

    let _whole = output.pull(2 * 48_000 + 2).ok_or("no output started")?;
    harness.step();
    assert!(
        harness.query_by_label("Play").is_some(),
        "stopped at the end"
    );
    press(&mut harness, "Play");
    let again = output.pull(8).ok_or("the stream ended")?;

    let expected: Vec<f32> = (0..4)
        .flat_map(|frame| [tone_sample(frame, 0), tone_sample(frame, 1)])
        .collect();
    assert!(
        again
            .iter()
            .zip(&expected)
            .all(|(&got, &want)| near(got, want))
            && again.iter().any(|&sample| sample != 0.0),
        "from the start: {again:?}"
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
    harness.key_press(Key::Home);
    harness.step();
    let silent = harness.state().audio_volume();
    harness.key_press(Key::ArrowRight);
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
fn a_failing_audio_output_shows_a_message_instead_of_playing() -> TestResult {
    let output = FakeOutput {
        failure: Some("no audio device"),
        ..FakeOutput::default()
    };
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
fn switching_to_the_text_tab_pauses_the_sound() -> TestResult {
    let output = FakeOutput::default();
    let (_folder, mut harness) = opened(Language::BritishEnglish, &output)?;
    select(&mut harness, "tone.wem")?;
    press(&mut harness, "Play");
    let _start = output.pull(2_000).ok_or("no output started")?;

    harness.state_mut().set_preview_tab(PreviewTab::Text);
    let silent = output.pull(8).ok_or("the stream ended")?;
    let paused_at = position(harness.state());
    harness.state_mut().set_preview_tab(PreviewTab::Audio);
    harness.step();
    press(&mut harness, "Play");
    let resumed = output.pull(8).ok_or("the stream ended")?;

    assert!(silent.iter().all(|&sample| sample == 0.0), "silence");
    assert!(paused_at > 0.0, "the position is kept: {paused_at}");
    let expected: Vec<f32> = (1_000..1_004)
        .flat_map(|frame| [tone_sample(frame, 0), tone_sample(frame, 1)])
        .collect();
    assert!(
        resumed
            .iter()
            .zip(&expected)
            .all(|(&got, &want)| near(got, want)),
        "resumes where it paused: {resumed:?}"
    );
    harness.state_mut().set_work_tab(WorkTab::Query);
    assert!(
        player(harness.state()).is_some_and(|player| !player.is_playing()),
        "leaving Browse pauses too"
    );
    Ok(())
}

#[test]
fn the_keyboard_seeks_and_space_plays() -> TestResult {
    let output = FakeOutput::default();
    let (_folder, mut harness) = opened(Language::BritishEnglish, &output)?;
    select(&mut harness, "tone.wem")?;

    harness.key_press(Key::Space);
    harness.step();
    let started = player(harness.state()).is_some_and(p4kx_gui::Player::is_playing);
    harness
        .get_by_role_and_label(Role::Slider, "Position")
        .focus();
    harness.step();
    harness.key_press(Key::End);
    harness.step();
    let end = position(harness.state());
    harness.key_press(Key::Home);
    harness.step();
    let start = position(harness.state());
    harness.key_press(Key::PageUp);
    harness.step();
    let paged = position(harness.state());
    harness.key_press(Key::Space);
    harness.step();
    let paused = player(harness.state()).is_some_and(p4kx_gui::Player::is_playing);

    assert!(started, "Space plays");
    assert!(
        near(end, 1.0) && near(start, 0.0),
        "End {end} and Home {start}"
    );
    assert!(
        near(paged, 1.0),
        "Page Up moves 30 s, past the end: {paged}"
    );
    assert!(!paused, "Space on the position bar pauses");
    Ok(())
}
