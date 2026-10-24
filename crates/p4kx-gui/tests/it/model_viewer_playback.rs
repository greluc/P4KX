// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The skeleton overlay and the animation playback of the 3D viewer: the controls and the state
//! behind them, headless.

use crate::headless::ready;
use crate::viewing::{Site, TestResult, model_preview, opened_at, select};

use std::error::Error;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model, ModelPreview, Playback};
use p4kx_testkit::{TestAnimation, TestEntry, TestModel, TestSkeleton, TestTrack};

const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const TURN: [f32; 4] = [0.0, 0.0, 0.707_106_77, 0.707_106_77];

fn tip() -> u32 {
    TestSkeleton::controller_id(1)
}

fn rotating(name: &str, controller: u32, last_frame: u16) -> TestAnimation {
    TestAnimation::new(name, 30)
        .track(TestTrack::new(controller).rotations(&[(0, IDENTITY), (last_frame, TURN)]))
}

pub(crate) fn site() -> Site {
    let arm = TestModel::two_bone_skin();
    let door = TestModel::cube()
        .moved([-10.0, 0.0, 0.0])
        .with_compact_skeleton(&TestSkeleton::chain(2));
    Site {
        entries: vec![
            TestEntry::zstd("Data\\Objects\\arm.skin", arm.cgf()),
            TestEntry::zstd("Data\\Objects\\arm.skinm", arm.cgfm()),
            TestEntry::zstd("Data\\Objects\\door.cga", door.cgf()),
            TestEntry::zstd("Data\\Objects\\door.cgam", door.cgfm()),
            TestEntry::zstd("Data\\Objects\\cube.cgf", TestModel::cube().cgf()),
            TestEntry::zstd("Data\\Objects\\cube.cgfm", TestModel::cube().cgfm()),
            TestEntry::zstd(
                "Data\\Objects\\swing.caf",
                rotating("swing.caf", tip(), 30).caf(),
            ),
            TestEntry::zstd(
                "Data\\Objects\\slow.caf",
                rotating("slow.caf", tip(), 90).caf(),
            ),
            TestEntry::zstd(
                "Data\\Objects\\other.caf",
                rotating("other.caf", 0xBAD0_0001, 30).caf(),
            ),
        ],
        installation: false,
        key: false,
    }
}

fn shown(
    language: Language,
    model: &str,
) -> Result<(tempfile::TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let (folder, mut harness) = opened_at(language, ready(), site(), |builder| builder)?;
    select(&mut harness, model)?;
    harness.run_steps(3);
    Ok((folder, harness))
}

fn playback<'a>(harness: &'a Harness<'_, Model>) -> Result<&'a Playback, Box<dyn Error>> {
    model_preview(harness.state())
        .and_then(ModelPreview::playback)
        .ok_or_else(|| "no playback".into())
}

fn exists(harness: &Harness<'_, Model>, label: &str) -> bool {
    harness.query_by_label(label).is_some()
}

#[test]
fn a_skinned_model_lists_the_animations_that_move_its_bones() -> TestResult {
    let (_folder, harness) = shown(Language::BritishEnglish, "arm.skin")?;

    let playback = playback(&harness)?;
    let names: Vec<_> = (0..playback.animations().len())
        .filter_map(|at| playback.name(at))
        .collect();
    assert_eq!(
        names,
        ["slow", "swing"],
        "the one of another skeleton is left out"
    );
    for label in [
        "Bind pose",
        "swing",
        "slow",
        "Animations",
        "Skeleton",
        "Loop",
        "Play",
    ] {
        assert!(exists(&harness, label), "{label} is shown");
    }
    assert!(
        !exists(&harness, "other"),
        "an unrelated animation is not listed"
    );
    Ok(())
}

#[test]
fn nothing_plays_until_an_animation_is_chosen() -> TestResult {
    let (_folder, harness) = shown(Language::BritishEnglish, "arm.skin")?;

    let playback = playback(&harness)?;
    assert_eq!(playback.selected(), None, "the bind pose shows");
    assert!(!playback.is_playing(), "not playing");
    assert!(!playback.shows_skeleton(), "no overlay");
    Ok(())
}

#[test]
fn choosing_an_animation_plays_it_and_time_moves_on() -> TestResult {
    let (_folder, mut harness) = shown(Language::BritishEnglish, "arm.skin")?;

    harness.get_by_label("slow").click();
    harness.run_steps(12);

    let playback = playback(&harness)?;
    assert_eq!(playback.selected(), Some(0), "selected");
    assert!(playback.is_playing(), "playing");
    assert!(playback.time() > 0.0, "time moved: {}", playback.time());
    assert!(exists(&harness, "Pause"), "the action offers Pause");
    Ok(())
}

#[test]
fn pause_holds_the_position_and_play_goes_on() -> TestResult {
    let (_folder, mut harness) = shown(Language::BritishEnglish, "arm.skin")?;
    harness.get_by_label("slow").click();
    harness.run_steps(2);

    harness.get_by_label("Pause").click();
    harness.run_steps(2);
    let held = playback(&harness)?.time();
    harness.run_steps(6);

    assert!(!playback(&harness)?.is_playing(), "paused");
    assert!(
        (playback(&harness)?.time() - held).abs() < f32::EPSILON,
        "held"
    );
    assert!(exists(&harness, "Play"), "the action offers Play");
    harness.get_by_label("Play").click();
    harness.run_steps(2);
    assert!(playback(&harness)?.is_playing(), "playing again");
    assert!(playback(&harness)?.time() > held, "went on");
    Ok(())
}

#[test]
fn the_loop_and_skeleton_checkboxes_switch_their_state() -> TestResult {
    let (_folder, mut harness) = shown(Language::BritishEnglish, "arm.skin")?;
    assert!(playback(&harness)?.is_looping(), "repeats at first");

    harness.get_by_label("Loop").click();
    harness.run_steps(2);
    harness.get_by_label("Skeleton").click();
    harness.run_steps(2);

    assert!(!playback(&harness)?.is_looping(), "no repeat");
    assert!(playback(&harness)?.shows_skeleton(), "skeleton shown");
    harness.get_by_label("Skeleton").click();
    harness.run_steps(2);
    assert!(!playback(&harness)?.shows_skeleton(), "hidden again");
    Ok(())
}

#[test]
fn the_timeline_scrubs_to_where_it_is_clicked() -> TestResult {
    let (_folder, mut harness) = shown(Language::BritishEnglish, "arm.skin")?;
    harness.get_by_label("slow").click();
    harness.run_steps(2);
    harness.get_by_label("Pause").click();
    harness.run_steps(2);

    harness.get_by_label("Animation position").click();
    harness.run_steps(3);

    let playback = playback(&harness)?;
    assert!(
        (playback.fraction() - 0.5).abs() < 0.2,
        "about halfway: {}",
        playback.fraction()
    );
    assert!(!playback.is_playing(), "scrubbing does not resume");
    Ok(())
}

#[test]
fn choosing_the_bind_pose_stops_the_animation() -> TestResult {
    let (_folder, mut harness) = shown(Language::BritishEnglish, "arm.skin")?;
    harness.get_by_label("swing").click();
    harness.run_steps(4);

    harness.get_by_label("Bind pose").click();
    harness.run_steps(2);

    let playback = playback(&harness)?;
    assert_eq!(playback.selected(), None, "no animation");
    assert!(!playback.is_playing(), "stopped");
    Ok(())
}

#[test]
fn a_static_model_with_bones_offers_the_skeleton_alone() -> TestResult {
    let (_folder, mut harness) = shown(Language::BritishEnglish, "door.cga")?;

    assert!(
        playback(&harness)?.animations().is_empty(),
        "no skin to animate"
    );
    assert!(
        exists(&harness, "Skeleton"),
        "the overlay can be switched on"
    );
    for label in ["Play", "Loop", "Animations", "Bind pose"] {
        assert!(!exists(&harness, label), "{label} is not shown");
    }
    harness.get_by_label("Skeleton").click();
    harness.run_steps(2);
    assert!(playback(&harness)?.shows_skeleton(), "shown");
    Ok(())
}

#[test]
fn a_model_without_bones_shows_no_playback_controls() -> TestResult {
    let (_folder, harness) = shown(Language::BritishEnglish, "cube.cgf")?;

    assert!(
        model_preview(harness.state())
            .and_then(ModelPreview::playback)
            .is_none(),
        "no skeleton, no playback"
    );
    for label in ["Skeleton", "Play", "Animations"] {
        assert!(!exists(&harness, label), "{label} is not shown");
    }
    Ok(())
}

#[test]
fn the_controls_speak_german() -> TestResult {
    let (_folder, harness) = shown(Language::German, "arm.skin")?;

    for label in [
        "Ausgangspose",
        "Animationen",
        "Skelett",
        "Wiederholen",
        "Abspielen",
    ] {
        assert!(exists(&harness, label), "{label} is shown");
    }
    Ok(())
}
