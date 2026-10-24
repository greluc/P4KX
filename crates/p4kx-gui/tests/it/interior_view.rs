// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Interior in the 3D view of the Entities tab: built once it is switched on, hidden and shown
//! again without building, kept across other choices and stopped on request.

use crate::entity_picking::{TestResult, Window, opened, opened_sized, select};
use crate::tagging::wait_until;

use egui_kittest::Harness;
use p4kx_gui::{AssemblyView, Camera, DrawList, Language, LoadoutSession, Model};
use p4kx_testkit::{ENTITY_INTERIOR_CLASS, entity_interior_entries};

fn hull_window() -> Result<Window, Box<dyn std::error::Error>> {
    opened_sized(
        entity_interior_entries(),
        Language::BritishEnglish,
        [1400.0, 900.0],
    )
}

fn view(model: &Model) -> Option<&AssemblyView> {
    model.entities().loadout().map(LoadoutSession::assembly)
}

fn view_mut<'a>(harness: &'a mut Harness<'_, Model>) -> Result<&'a mut AssemblyView, &'static str> {
    harness
        .state_mut()
        .entities_mut()
        .loadout_mut()
        .map(LoadoutSession::assembly_mut)
        .ok_or("no Loadout")
}

fn serial(model: &Model) -> Option<u64> {
    view(model)
        .and_then(AssemblyView::scene)
        .map(|scene| scene.serial())
}

fn built(harness: &mut Harness<'_, Model>, before: Option<u64>) -> TestResult {
    wait_until(harness, |model| {
        view(model).is_some_and(|view| !view.is_building())
            && serial(model).is_some()
            && serial(model) != before
    })
}

fn interior_parts(model: &Model) -> usize {
    view(model)
        .and_then(AssemblyView::scene)
        .map_or(0, |scene| {
            scene
                .parts()
                .iter()
                .filter(|part| part.is_interior())
                .count()
        })
}

fn instances(harness: &mut Harness<'_, Model>) -> Result<usize, Box<dyn std::error::Error>> {
    let view = view_mut(harness)?;
    let mut camera: Camera = view.camera().cloned().ok_or("no camera")?;
    camera.fit(1.5);
    let matrix = camera.uniforms(1.5).view_proj;
    let list: std::sync::Arc<DrawList> = view.draw_list(&matrix, 800.0).ok_or("no scene")?;
    Ok(list.instances().len())
}

#[test]
fn an_entity_with_object_containers_offers_the_interior_and_builds_it_once_it_is_switched_on()
-> TestResult {
    let (_dir, mut harness) = hull_window()?;
    select(&mut harness, ENTITY_INTERIOR_CLASS)?;
    built(&mut harness, None)?;
    let first = serial(harness.state());

    let shown = view(harness.state()).ok_or("no view")?;
    assert!(shown.has_interior(), "the Record names object containers");
    assert!(!shown.interior(), "the view starts with the hull alone");
    assert_eq!(interior_parts(harness.state()), 0, "no part of it yet");

    view_mut(&mut harness)?.set_interior(true);
    let building = view_mut(&mut harness)?;
    assert!(building.is_building(), "built off the UI thread");
    assert!(building.progress().is_some(), "with progress");
    assert!(building.is_chosen(), "a choice of the user");
    built(&mut harness, first)?;

    assert!(interior_parts(harness.state()) > 0, "the rooms are parts");
    let scene = view(harness.state())
        .and_then(AssemblyView::scene)
        .ok_or("no scene")?;
    assert!(scene.interior_parts() > 0, "and they are drawn");
    assert!(
        scene.drawn(true).0 > scene.drawn(false).0,
        "more parts with the Interior"
    );
    assert!(
        view(harness.state()).is_some_and(|view| view.interior() && view.failure().is_none()),
        "shown"
    );
    Ok(())
}

#[test]
fn switching_the_interior_off_and_on_hides_and_shows_it_without_building_again() -> TestResult {
    let (_dir, mut harness) = hull_window()?;
    select(&mut harness, ENTITY_INTERIOR_CLASS)?;
    built(&mut harness, None)?;
    view_mut(&mut harness)?.set_interior(true);
    let before = serial(harness.state());
    built(&mut harness, before)?;
    let scene = serial(harness.state());
    let all = instances(&mut harness)?;

    view_mut(&mut harness)?.set_interior(false);

    assert!(!view(harness.state()).is_some_and(AssemblyView::is_building));
    assert_eq!(serial(harness.state()), scene, "the same scene");
    let hull = instances(&mut harness)?;
    assert!(hull < all, "the Interior is left out: {hull} of {all}");
    assert_eq!(
        view(harness.state()).map(AssemblyView::hidden_parts),
        Some(0),
        "the parts of the Interior are not counted as hidden branches"
    );

    view_mut(&mut harness)?.set_interior(true);

    assert!(!view(harness.state()).is_some_and(AssemblyView::is_building));
    assert_eq!(serial(harness.state()), scene, "still the same scene");
    assert_eq!(instances(&mut harness)?, all, "shown again");
    Ok(())
}

#[test]
fn another_detail_builds_the_interior_again_only_while_it_is_shown() -> TestResult {
    let (_dir, mut harness) = hull_window()?;
    select(&mut harness, ENTITY_INTERIOR_CLASS)?;
    built(&mut harness, None)?;
    view_mut(&mut harness)?.set_interior(true);
    let before = serial(harness.state());
    built(&mut harness, before)?;

    let before = serial(harness.state());
    view_mut(&mut harness)?.set_lod(0);
    built(&mut harness, before)?;
    assert!(
        interior_parts(harness.state()) > 0,
        "the Interior comes with the new level"
    );

    view_mut(&mut harness)?.set_interior(false);
    let before = serial(harness.state());
    view_mut(&mut harness)?.set_lod(1);
    built(&mut harness, before)?;
    assert_eq!(
        interior_parts(harness.state()),
        0,
        "not built while it is off"
    );

    let before = serial(harness.state());
    view_mut(&mut harness)?.set_interior(true);
    built(&mut harness, before)?;
    assert!(interior_parts(harness.state()) > 0, "built when asked for");
    Ok(())
}

#[test]
fn an_entity_without_object_containers_has_no_interior_to_switch_on() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "TEST_Ship")?;
    built(&mut harness, None)?;

    view_mut(&mut harness)?.set_interior(true);

    let view = view(harness.state()).ok_or("no view")?;
    assert!(!view.has_interior(), "the Record names none");
    assert!(!view.interior(), "so it stays off");
    assert!(!view.is_building(), "and nothing is built");
    assert!(!view.is_chosen(), "and nothing was chosen");
    Ok(())
}

#[test]
fn stopping_the_interior_keeps_the_view_built_before() -> TestResult {
    let (_dir, mut harness) = hull_window()?;
    select(&mut harness, ENTITY_INTERIOR_CLASS)?;
    built(&mut harness, None)?;
    let first = serial(harness.state());

    let view = view_mut(&mut harness)?;
    view.set_interior(true);
    assert!(view.is_building(), "building");
    view.cancel();

    assert!(!view.is_building(), "stopped");
    assert!(view.is_cancelled(), "by the user");
    harness.run_steps(3);
    assert_eq!(serial(harness.state()), first, "the hull stays");
    assert_eq!(interior_parts(harness.state()), 0, "without the Interior");
    Ok(())
}

#[test]
fn the_interior_stays_on_when_the_view_is_built_again() -> TestResult {
    let (_dir, mut harness) = hull_window()?;
    select(&mut harness, ENTITY_INTERIOR_CLASS)?;
    built(&mut harness, None)?;
    view_mut(&mut harness)?.set_interior(true);
    let before = serial(harness.state());
    built(&mut harness, before)?;

    let before = serial(harness.state());
    view_mut(&mut harness)?.restart();
    built(&mut harness, before)?;

    assert!(
        view(harness.state()).is_some_and(AssemblyView::interior),
        "still switched on"
    );
    assert!(interior_parts(harness.state()) > 0, "and built");
    Ok(())
}
