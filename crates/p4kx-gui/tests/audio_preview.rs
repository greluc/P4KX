//! The Audio tab of the preview over a synthetic Archive with WEMs, driven headless through a
//! fake audio output.

mod sounding;

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{AudioStatus, Language, Model, PreviewTab, SoundStatus};
use p4kx_testkit::TestVorbis;
use sounding::{FakeOutput, TestResult, audio, opened, player, select};

fn press(harness: &mut egui_kittest::Harness<'_, Model>, label: &str) {
    harness.get_by_label(label).click();
    for _ in 0..3 {
        harness.step();
    }
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
fn ima_adpcm_and_ptadpcm_decode_and_play() -> TestResult {
    for (name, codec) in [("ima.WEM", "IMA ADPCM"), ("platinum.wem", "PTADPCM")] {
        let output = FakeOutput::default();
        let (_folder, mut harness) = opened(Language::BritishEnglish, &output)?;

        select(&mut harness, name)?;
        press(&mut harness, "Play");

        let facts = format!("Codec {codec} Channels 1 Sample rate 48,000 Hz");
        assert!(
            harness.query_by_label_contains(&facts).is_some(),
            "facts of {name}"
        );
        assert_eq!(output.started(), 1, "{name} plays");
        assert!(
            output
                .pull(256)
                .is_some_and(|samples| samples.iter().any(|&s| s != 0.0)),
            "{name} is not silent"
        );
    }
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
        Some(u64::from(
            TestVorbis::new(2, 48_000, &[false; 400])
                .tone(10)
                .decoded_samples(),
        )),
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
