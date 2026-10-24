// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The controls of the 3D viewer: orbit, pan, zoom and reset with the mouse and the keyboard.

use crate::headless;
use crate::viewing;

use egui::accesskit::Role;
use std::error::Error;

use egui::{Event, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, TouchPhase, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use headless::{opened, ready, status_is_ready};
use p4kx_gui::{Camera, Language, Model, ModelPreview};
use viewing::{TestResult, model_preview, select};

fn camera(harness: &Harness<'_, Model>) -> Result<Camera, Box<dyn Error>> {
    model_preview(harness.state())
        .and_then(ModelPreview::camera)
        .cloned()
        .ok_or_else(|| "no camera".into())
}

fn drag(harness: &mut Harness<'_, Model>, from: Pos2, to: Pos2, button: PointerButton) {
    harness.event(Event::PointerMoved(from));
    harness.step();
    let press = |pos, pressed| Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.event(press(from, true));
    harness.step();
    harness.event(Event::PointerMoved(Pos2::new(
        f32::midpoint(from.x, to.x),
        f32::midpoint(from.y, to.y),
    )));
    harness.step();
    harness.event(Event::PointerMoved(to));
    harness.step();
    harness.event(press(to, false));
    harness.run_steps(2);
}

const VIEW: &str = "3D view of Data\\Objects\\cube.cgf: 24 vertices, 12 triangles";

fn shown() -> Result<(tempfile::TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let (folder, mut harness) = opened(Language::BritishEnglish, ready())?;
    select(&mut harness, "cube.cgf")?;
    harness.run_steps(3);
    assert!(status_is_ready(&harness), "the model is read");
    Ok((folder, harness))
}

fn centre(harness: &Harness<'_, Model>) -> Pos2 {
    harness
        .get_by_role_and_label(Role::Canvas, VIEW)
        .rect()
        .center()
}

fn reset_is_disabled(harness: &Harness<'_, Model>) -> bool {
    harness
        .get_by_label("Reset view")
        .accesskit_node()
        .is_disabled()
}

fn wheel(harness: &Harness<'_, Model>, delta: f32) {
    harness.hover_at(centre(harness));
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, delta),
        phase: TouchPhase::Move,
        modifiers: Modifiers::NONE,
        source: egui::MouseWheelSource::Wheel,
    });
}

fn moved(before: &Camera, after: &Camera) -> bool {
    before != after
}

#[test]
fn the_view_starts_framed_and_the_reset_action_waits_for_a_change() -> TestResult {
    let (_folder, harness) = shown()?;

    assert!(camera(&harness)?.is_fitted(), "framed to fit");
    assert!(reset_is_disabled(&harness), "nothing to reset yet");
    Ok(())
}

#[test]
fn dragging_orbits_around_the_model() -> TestResult {
    let (_folder, mut harness) = shown()?;
    let before = camera(&harness)?;
    let from = centre(&harness);

    drag(
        &mut harness,
        from,
        from + vec2(60.0, 20.0),
        PointerButton::Primary,
    );

    let after = camera(&harness)?;
    assert!(moved(&before, &after), "the camera moved");
    assert_eq!(
        after.target(),
        before.target(),
        "still looking at the model"
    );
    assert!(
        (after.distance() - before.distance()).abs() < 1.0e-3,
        "same distance"
    );
    assert!(!reset_is_disabled(&harness), "reset is offered");
    Ok(())
}

#[test]
fn a_secondary_drag_pans() -> TestResult {
    let (_folder, mut harness) = shown()?;
    let before = camera(&harness)?;
    let from = centre(&harness);

    drag(
        &mut harness,
        from,
        from + vec2(-40.0, 15.0),
        PointerButton::Secondary,
    );

    let after = camera(&harness)?;
    assert_ne!(after.target(), before.target(), "the target moved");
    assert!(
        (after.distance() - before.distance()).abs() < 1.0e-3,
        "same distance"
    );
    Ok(())
}

#[test]
fn the_wheel_zooms_in_and_out() -> TestResult {
    let (_folder, mut harness) = shown()?;
    let start = camera(&harness)?.distance();

    wheel(&harness, 80.0);
    harness.run_steps(2);
    let closer = camera(&harness)?.distance();
    wheel(&harness, -160.0);
    harness.run_steps(2);
    let farther = camera(&harness)?.distance();

    assert!(closer < start, "scrolling up zooms in: {start} {closer}");
    assert!(
        farther > closer,
        "scrolling down zooms out: {closer} {farther}"
    );
    Ok(())
}

#[test]
fn the_reset_action_frames_the_model_again() -> TestResult {
    let (_folder, mut harness) = shown()?;
    let start = camera(&harness)?;
    let from = centre(&harness);
    drag(
        &mut harness,
        from,
        from + vec2(50.0, 30.0),
        PointerButton::Primary,
    );
    wheel(&harness, 120.0);
    harness.run_steps(2);
    assert!(moved(&start, &camera(&harness)?), "changed");

    harness.get_by_label("Reset view").click();
    harness.run_steps(3);

    assert_eq!(camera(&harness)?, start, "back at the start");
    assert!(reset_is_disabled(&harness), "nothing left to reset");
    Ok(())
}

#[test]
fn the_keyboard_orbits_pans_zooms_and_resets_the_focused_view() -> TestResult {
    let (_folder, mut harness) = shown()?;
    let start = camera(&harness)?;
    harness.get_by_role_and_label(Role::Canvas, VIEW).focus();
    harness.run_steps(2);

    harness.key_press(Key::ArrowLeft);
    harness.run_steps(2);
    let orbited = camera(&harness)?;
    assert_ne!(orbited.eye(), start.eye(), "the arrow keys orbit");
    assert_eq!(orbited.target(), start.target(), "around the same point");

    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
    harness.run_steps(2);
    let panned = camera(&harness)?;
    assert_ne!(panned.target(), orbited.target(), "Shift and an arrow pan");

    harness.key_press(Key::Plus);
    harness.run_steps(2);
    let closer = camera(&harness)?.distance();
    harness.key_press(Key::Minus);
    harness.key_press(Key::Minus);
    harness.run_steps(2);
    let farther = camera(&harness)?.distance();
    assert!(closer < panned.distance(), "plus zooms in");
    assert!(farther > closer, "minus zooms out");

    harness.key_press(Key::Home);
    harness.run_steps(2);
    assert_eq!(camera(&harness)?, start, "Home resets");
    Ok(())
}

#[test]
fn the_keys_do_nothing_while_the_view_is_not_focused() -> TestResult {
    let (_folder, mut harness) = shown()?;
    let start = camera(&harness)?;

    harness.key_press(Key::ArrowLeft);
    harness.key_press(Key::Plus);
    harness.run_steps(2);

    assert_eq!(camera(&harness)?, start, "unchanged");
    Ok(())
}

#[test]
fn another_model_starts_with_its_own_framed_camera() -> TestResult {
    let (_folder, mut harness) = shown()?;
    let from = centre(&harness);
    drag(
        &mut harness,
        from,
        from + vec2(50.0, 30.0),
        PointerButton::Primary,
    );
    assert!(!reset_is_disabled(&harness), "moved");

    harness.get_by_label("readme.txt").click();
    harness.run_steps(3);
    harness.get_by_label("cube.cgf").click();
    viewing::wait_until(&mut harness, viewing::settled)?;
    harness.run_steps(3);

    assert!(reset_is_disabled(&harness), "a fresh camera");
    Ok(())
}
