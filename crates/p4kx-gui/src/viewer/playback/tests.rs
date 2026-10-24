// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use super::{Control, Playback};
use crate::viewer::fixtures::{skin_model, swing};
use p4kx_render::Skeleton;

fn playback() -> Playback {
    let model = skin_model();
    let skeleton = Skeleton::of(&model.bones).map(Arc::new);
    Playback::new(
        skeleton.unwrap_or_else(|| panic!("a skeleton")),
        vec![swing("swing", 30), swing("slow", 90)],
    )
}

#[test]
fn a_new_playback_shows_the_bind_pose_with_the_skeleton_hidden() {
    let playback = playback();

    assert_eq!(playback.selected(), None, "nothing selected");
    assert!(!playback.is_playing(), "not playing");
    assert!(!playback.shows_skeleton(), "no overlay");
    assert!(playback.is_looping(), "repeats by default");
    assert_eq!(playback.animations().len(), 2, "both listed");
}

#[test]
fn selecting_an_animation_starts_it_from_the_beginning() {
    let mut playback = playback();

    playback.apply(Control::Select(Some(1)));

    assert_eq!(playback.selected(), Some(1), "selected");
    assert!(playback.is_playing(), "playing");
    assert!(
        (playback.duration() - 3.0).abs() < 1.0e-5,
        "90 frames at 30 per second"
    );
    assert!(playback.time().abs() < f32::EPSILON, "at the start");
}

#[test]
fn selecting_none_or_a_missing_index_returns_to_the_bind_pose() {
    let mut playback = playback();
    playback.select(Some(0));

    playback.select(Some(7));

    assert_eq!(playback.selected(), None, "no such animation");
    assert!(!playback.is_playing(), "stopped");
    assert!(playback.duration().abs() < f32::EPSILON, "no length");
}

#[test]
fn play_and_pause_toggle_and_a_pause_holds_the_time() {
    let mut playback = playback();
    playback.select(Some(0));
    playback.tick(0.25);

    playback.apply(Control::Toggle);
    let held = playback.time();
    playback.tick(0.25);

    assert!(!playback.is_playing(), "paused");
    assert!((playback.time() - held).abs() < f32::EPSILON, "time held");
    playback.apply(Control::Toggle);
    assert!(playback.is_playing(), "playing again");
}

#[test]
fn toggling_without_a_selection_does_nothing() {
    let mut playback = playback();

    playback.apply(Control::Toggle);

    assert!(!playback.is_playing(), "nothing to play");
}

#[test]
fn time_advances_by_the_frame_time_and_a_stall_is_capped() {
    let mut playback = playback();
    playback.select(Some(1));

    playback.tick(0.1);
    assert!((playback.time() - 0.1).abs() < 1.0e-6, "one frame");
    playback.tick(10.0);

    assert!(
        (playback.time() - 0.35).abs() < 1.0e-6,
        "a stall advances a quarter second"
    );
}

#[test]
fn a_looping_animation_wraps_around_at_its_end() {
    let mut playback = playback();
    playback.select(Some(0));

    for _ in 0..5 {
        playback.tick(0.25);
    }

    assert!(playback.is_playing(), "still playing");
    assert!(playback.time() < playback.duration(), "wrapped");
    assert!(
        (playback.time() - 0.25).abs() < 1.0e-5,
        "5 × 0.25 s over 1 s"
    );
}

#[test]
fn an_animation_without_looping_stops_at_its_end_and_restarts_on_play() {
    let mut playback = playback();
    playback.apply(Control::Loop(false));
    playback.select(Some(0));

    for _ in 0..6 {
        playback.tick(0.25);
    }

    assert!(!playback.is_playing(), "stopped");
    assert!(
        (playback.fraction() - 1.0).abs() < f32::EPSILON,
        "at the end"
    );
    playback.apply(Control::Toggle);
    assert!(playback.is_playing(), "plays again");
    assert!(playback.time().abs() < f32::EPSILON, "from the start");
}

#[test]
fn seeking_moves_the_time_and_keeps_playing_state_and_clamps() {
    let mut playback = playback();
    playback.select(Some(1));
    playback.apply(Control::Toggle);

    playback.apply(Control::Seek(0.5));
    assert!(
        (playback.time() - 1.5).abs() < 1.0e-5,
        "half of three seconds"
    );
    assert!(!playback.is_playing(), "the toggle paused it");
    playback.apply(Control::Seek(7.0));
    assert!(
        (playback.fraction() - 1.0).abs() < f32::EPSILON,
        "clamped to the end"
    );
    playback.apply(Control::Seek(f32::NAN));
    assert!(
        playback.time().abs() < f32::EPSILON,
        "not a number goes to the start"
    );
}

#[test]
fn seeking_without_a_selection_changes_nothing() {
    let mut playback = playback();

    playback.apply(Control::Seek(0.5));

    assert!(playback.time().abs() < f32::EPSILON, "no time");
}

#[test]
fn the_pose_changes_only_when_the_animation_moves() {
    let mut playback = playback();
    playback.select(Some(0));
    playback.tick(0.0);
    let first = playback.pose().serial();

    playback.apply(Control::Toggle);
    playback.tick(0.1);
    let paused = playback.pose().serial();
    playback.tick(0.1);

    assert_eq!(
        playback.pose().serial(),
        paused,
        "a paused animation keeps its pose"
    );
    assert_eq!(first, paused, "and a pause computes nothing");
    playback.apply(Control::Seek(0.5));
    playback.tick(0.0);
    assert_ne!(playback.pose().serial(), paused, "a seek poses again");
}

#[test]
fn a_playing_animation_computes_a_pose_every_frame_without_new_buffers() {
    let mut playback = playback();
    playback.select(Some(1));
    playback.tick(0.0);
    let before = playback.pose().matrices().as_ptr();

    for _ in 0..20 {
        playback.tick(0.016);
    }

    assert_eq!(
        playback.pose().matrices().as_ptr(),
        before,
        "the same buffer"
    );
}

#[test]
fn the_skeleton_toggle_does_not_touch_the_animation() {
    let mut playback = playback();
    playback.select(Some(0));

    playback.apply(Control::Skeleton(true));

    assert!(playback.shows_skeleton(), "shown");
    assert!(playback.is_playing(), "still playing");
}

#[test]
fn the_selected_animation_poses_the_tip_away_from_the_bind_pose() {
    let mut playback = playback();
    playback.select(Some(0));

    playback.apply(Control::Seek(1.0));
    playback.tick(0.0);

    let identity: [f32; 16] = std::array::from_fn(|at| if at % 5 == 0 { 1.0 } else { 0.0 });
    let matrices = playback.pose().matrices();
    let moved = matrices
        .last()
        .is_some_and(|tip| tip.iter().zip(identity).any(|(a, b)| (a - b).abs() > 0.1));
    assert!(moved, "the tip is posed");
}
