// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The language choice, the soundbank kept for playing and the note about a missing player of the
//! Sounds tab.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, ListStatus, Model, SoundsListStatus};

use crate::entity_sounds::{TestResult, opened, wait_until};
use crate::sounding::FakeOutput;

const SIZE: [f32; 2] = [1400.0, 800.0];
const BANK: &str = "Data\\Sounds\\wwise\\Test.bnk";

fn resolved(harness: &mut Harness<'_, Model>, query: &str) -> TestResult {
    harness.state_mut().sounds_mut().set_query(query);
    wait_until(harness, |model| {
        matches!(model.sounds().list_status(), ListStatus::Ready(_))
    })?;
    harness.state_mut().sounds_mut().select(0);
    wait_until(harness, |model| {
        matches!(model.sounds().sounds_status(), SoundsListStatus::Ready(_))
    })
}

fn languages(harness: &Harness<'_, Model>) -> Vec<(Option<String>, bool)> {
    match harness.state().sounds().sounds_status() {
        SoundsListStatus::Ready(list) => list
            .variants
            .iter()
            .map(|variant| (variant.language.clone(), variant.fallback))
            .collect(),
        _ => Vec::new(),
    }
}

fn choose(harness: &mut Harness<'_, Model>, label: &str) -> TestResult {
    let before = harness.state().sounds().sounds_serial();
    harness.get_by_role_and_label(Role::Button, label).click();
    harness.run_steps(2);
    wait_until(harness, |model| {
        model.sounds().sounds_serial() > before
            && matches!(model.sounds().sounds_status(), SoundsListStatus::Ready(_))
    })
}

#[test]
fn the_language_buttons_resolve_the_selected_trigger_in_the_chosen_language() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish, SIZE, &FakeOutput::default())?;
    resolved(&mut harness, "play_voice")?;
    harness.state_mut().select_sound_variant(0, || {});

    assert_eq!(
        languages(&harness),
        [(Some("English(US)".to_owned()), false)],
        "the default language"
    );
    assert_eq!(harness.state().sounds().language(), None, "none chosen");

    choose(&mut harness, "German")?;

    assert_eq!(harness.state().sounds().language(), Some("German"));
    assert_eq!(
        languages(&harness),
        [(Some("German".to_owned()), false)],
        "resolved in German"
    );
    assert_eq!(
        harness.state().sounds().selected_variant(),
        None,
        "the playing sound of the other language is dropped"
    );

    choose(&mut harness, "English(US)")?;

    assert_eq!(harness.state().sounds().language(), None, "default again");
    assert_eq!(
        languages(&harness),
        [(Some("English(US)".to_owned()), false)],
        "back in the default language"
    );
    Ok(())
}

#[test]
fn a_language_that_has_no_sound_falls_back_and_says_so() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish, SIZE, &FakeOutput::default())?;
    resolved(&mut harness, "play_voice")?;

    harness
        .state_mut()
        .sounds_mut()
        .set_language(Some("Klingon"));
    wait_until(&mut harness, |model| {
        matches!(model.sounds().sounds_status(), SoundsListStatus::Ready(_))
    })?;

    assert_eq!(
        languages(&harness),
        [(Some("English(US)".to_owned()), true)],
        "the default language stands in"
    );
    Ok(())
}

#[test]
fn the_soundbank_of_a_played_sound_is_kept_for_the_next_one() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish, SIZE, &FakeOutput::default())?;
    resolved(&mut harness, "play_multi")?;
    let id = harness
        .state()
        .browser()
        .and_then(|browser| browser.index().id_of_path(BANK))
        .ok_or("no soundbank Entry")?;
    assert_eq!(
        harness.state().sounds().kept_soundbank(),
        None,
        "nothing yet"
    );

    harness.state_mut().select_sound_variant(0, || {});
    wait_until(&mut harness, |model| {
        model
            .sounds()
            .audio()
            .is_some_and(|audio| audio.player().is_some())
    })?;

    assert_eq!(
        harness.state().sounds().kept_soundbank(),
        Some(id),
        "kept after the first embedded sound"
    );
    Ok(())
}
