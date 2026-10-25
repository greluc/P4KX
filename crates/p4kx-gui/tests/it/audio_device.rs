// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Playing through the system's default audio device: whether one exists or not, the window
//! stays responsive and either plays or names why it cannot.

use crate::waiting::wait_for_model;
use std::error::Error;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{AudioPreview, Browser, Language, Model, Player, Preview, ui};
use p4kx_testkit::{ArchiveBuilder, TestCodec, TestEntry, TestWem, test_signal};

type TestResult = Result<(), Box<dyn Error>>;

fn player(model: &Model) -> Option<&Player> {
    model
        .browser()
        .and_then(Browser::preview)
        .and_then(Preview::audio)
        .and_then(AudioPreview::player)
}

fn settle(harness: &mut Harness<'_, Model>, done: impl Fn(&Model) -> bool) -> TestResult {
    wait_for_model(harness, "the window to settle", done)?;
    harness.step();
    Ok(())
}

const DEVICE_WAIT: Duration = Duration::from_secs(10);
const FRAME_LIMIT: Duration = Duration::from_millis(500);

#[test]
fn the_default_device_plays_or_names_why_it_cannot() -> TestResult {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    let tone = TestWem::pcm(TestCodec::Pcm, 2, 48_000, &test_signal(48_000, 2)).bytes();
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Sounds\\tone.wem", tone))
        .write_to(&path)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.set_audio_volume(0.0);
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1600.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.state_mut().open(path, || {});
    settle(&mut harness, |model| model.browser().is_some())?;
    harness.get_by_label("tone.wem").click();
    settle(&mut harness, |model| player(model).is_some())?;

    harness.get_by_label("Play").click();
    let deadline = Instant::now() + DEVICE_WAIT;
    let mut slowest = Duration::ZERO;
    let mut settled = false;
    while Instant::now() < deadline && !settled {
        let started = Instant::now();
        harness.step();
        slowest = slowest.max(started.elapsed());
        let failed = player(harness.state()).and_then(Player::failure);
        let moved = player(harness.state()).is_some_and(|player| player.fraction() > 0.0);
        settled = failed.is_some() || moved;
        std::thread::sleep(Duration::from_millis(10));
    }
    harness.step();

    let failure = player(harness.state()).and_then(Player::failure);
    if let Some(reason) = failure {
        let message = format!("The sound cannot be played: {reason}");
        assert!(harness.query_by_label(&message).is_some(), "message shown");
    }
    assert!(settled, "the sound played or the output failed");
    assert!(slowest < FRAME_LIMIT, "the slowest frame took {slowest:?}");
    Ok(())
}
