// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The expression preview of the Characters tab: the expressions of a head are loaded on demand
//! and one of them is applied to the face in the view.

use std::sync::Arc;

use crate::characters::{TestResult, Window, built, opened, pick, settle, wait_until};
use p4kx_gui::{ChooserKind, Language, Scene};

fn with_head() -> Result<Window, Box<dyn std::error::Error>> {
    let (dir, mut harness) = opened(Language::BritishEnglish)?;
    pick(&mut harness, ChooserKind::Head, "Runner.chf")?;
    wait_until(&mut harness, |model| {
        model.characters().head().is_some() && built(model)
    })?;
    Ok((dir, harness))
}

fn scene(harness: &Window) -> Result<Arc<Scene>, &'static str> {
    harness
        .1
        .state()
        .model
        .characters()
        .scene()
        .cloned()
        .ok_or("no scene")
}

fn moved(before: &Scene, after: &Scene) -> Vec<f32> {
    before
        .vertices()
        .iter()
        .zip(after.vertices())
        .filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| (x - y).abs() > 0.0))
        .map(|(a, b)| {
            (b[0] - a[0])
                .abs()
                .max((b[1] - a[1]).abs())
                .max((b[2] - a[2]).abs())
        })
        .collect()
}

#[test]
fn the_expressions_are_not_loaded_until_asked_for() -> TestResult {
    let window = with_head()?;

    let tab = window.1.state().model.characters();

    assert!(!tab.expressions_enabled(), "off");
    assert!(tab.expressions().is_empty(), "no morph targets are made");
    Ok(())
}

#[test]
#[expect(clippy::too_many_lines, reason = "one scenario in three steps")]
fn an_expression_moves_the_face_by_its_strength_and_the_neutral_face_restores_it() -> TestResult {
    let mut window = with_head()?;
    window
        .1
        .state_mut()
        .model
        .characters_mut()
        .set_expressions_enabled(true);
    settle(&mut window.1)?;
    let names: Vec<String> = window
        .1
        .state()
        .model
        .characters()
        .expressions()
        .iter()
        .map(|expression| expression.name().to_owned())
        .collect();
    assert_eq!(
        names,
        ["CTL_a", "CTL_b"],
        "the blendshapes that move vertices"
    );
    let neutral = scene(&window)?;

    window
        .1
        .state_mut()
        .model
        .characters_mut()
        .choose_expression(Some(0));
    let full = scene(&window)?;
    let full_moves = moved(&neutral, &full);
    assert!(!full_moves.is_empty(), "the face moved");
    assert_ne!(
        full.serial(),
        neutral.serial(),
        "a new scene for the device"
    );
    assert_eq!(full.vertices().len(), neutral.vertices().len(), "same mesh");
    assert!(
        full.skin().is_some() && full.skeleton().is_some(),
        "still skinned"
    );

    window
        .1
        .state_mut()
        .model
        .characters_mut()
        .set_expression_weight(0.5);
    let half = scene(&window)?;
    let half_moves = moved(&neutral, &half);
    assert_eq!(half_moves.len(), full_moves.len(), "the same vertices");
    for (half, full) in half_moves.iter().zip(&full_moves) {
        assert!(
            (half * 2.0 - full).abs() < 1.0e-6,
            "half the way: {half} {full}"
        );
    }

    window
        .1
        .state_mut()
        .model
        .characters_mut()
        .choose_expression(None);
    let back = scene(&window)?;
    assert!(moved(&neutral, &back).is_empty(), "the neutral face");
    Ok(())
}

#[test]
fn turning_the_expressions_off_drops_them_and_the_choice() -> TestResult {
    let mut window = with_head()?;
    let tab = window.1.state_mut().model.characters_mut();
    tab.set_expressions_enabled(true);
    settle(&mut window.1)?;
    let tab = window.1.state_mut().model.characters_mut();
    tab.choose_expression(Some(1));
    assert_eq!(tab.expression(), Some(1), "chosen");

    tab.set_expressions_enabled(false);
    settle(&mut window.1)?;

    let tab = window.1.state().model.characters();
    assert!(tab.expressions().is_empty(), "dropped");
    assert_eq!(tab.expression(), None, "the choice too");
    Ok(())
}

#[test]
fn a_strength_is_kept_between_nothing_and_full() -> TestResult {
    let mut window = with_head()?;
    let tab = window.1.state_mut().model.characters_mut();

    tab.set_expression_weight(7.0);
    assert!((tab.expression_weight() - 1.0).abs() < f32::EPSILON, "full");
    tab.set_expression_weight(-1.0);
    assert!(tab.expression_weight().abs() < f32::EPSILON, "none");
    tab.set_expression_weight(f32::NAN);
    assert!(
        (tab.expression_weight() - 1.0).abs() < f32::EPSILON,
        "reset"
    );
    Ok(())
}
