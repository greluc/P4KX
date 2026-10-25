// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The UI tab over the screens of a ship: the picker, the screens of the Loadout and their
//! pictures, driven headless over a synthetic Archive.

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, ShipPick, UiMode, UiScreensStatus, UiSource, UiText};

use crate::tagging::wait_until;
use crate::ui_tab_files::pixel;
use crate::ui_tabbing::{TestResult, drawn, opened, pick_ship};

const WINDOW: [f32; 2] = [1280.0, 800.0];

fn near(actual: Option<[u8; 4]>, expected: [u8; 4]) -> bool {
    actual.is_some_and(|actual| {
        actual
            .iter()
            .zip(expected)
            .all(|(actual, expected)| actual.abs_diff(expected) <= 2)
    })
}

#[test]
fn the_ships_segment_lists_the_ships_and_vehicles_of_the_archive() -> TestResult {
    let mut window = opened(Language::BritishEnglish, WINDOW)?;

    window
        .harness
        .get_by_role_and_label(Role::Button, "Ships")
        .click();
    wait_until(&mut window.harness, |model| model.ui().ships().is_some())?;

    assert_eq!(window.harness.state().ui().mode(), UiMode::Ships, "mode");
    let ships = window.harness.state().ui().ships().ok_or("no ships")?;
    assert_eq!(ships.total(), 1, "only the ship, not its screen Entities");
    assert_eq!(
        ships.get(0).map(p4kx_assembly::EntitySummary::class_name),
        Some("TEST_ScreenShip"),
        "the ship"
    );
    assert!(
        window.harness.query_by_label("TEST_ScreenShip").is_some(),
        "its row"
    );
    assert!(
        window.harness.query_by_label("1 ship or vehicle").is_some(),
        "the footer"
    );
    assert!(
        window
            .harness
            .query_by_label("Pick a ship to list its screens.")
            .is_some(),
        "asks to pick"
    );
    Ok(())
}

#[test]
fn a_picked_ship_lists_its_screens_and_draws_the_first() -> TestResult {
    let mut window = opened(Language::BritishEnglish, WINDOW)?;

    pick_ship(&mut window)?;

    let model = window.harness.state();
    let ship = model.ui().ship().ok_or("no ship")?;
    assert_eq!(ship.class_name(), "TEST_ScreenShip", "the ship");
    let UiScreensStatus::Ready(listing) = ship.status() else {
        return Err("the screens are not listed".into());
    };
    let ports: Vec<&str> = listing
        .screens
        .iter()
        .map(|screen| screen.port.as_str())
        .collect();
    assert_eq!(ports, ["Left_MFD", "Radar"], "one screen per port");
    assert_eq!(ship.picked(), Some(0), "the first screen is picked");
    let rendered = model.ui().rendered().ok_or("no picture")?;
    assert_eq!((rendered.width(), rendered.height()), (200, 100), "canvas");
    assert!(
        near(pixel(model, 100, 90), [10, 16, 24, 255]),
        "the background of the brand style: {:?}",
        pixel(model, 100, 90)
    );
    for label in ["Left_MFD", "Radar"] {
        assert!(
            window.harness.query_all_by_label(label).count() >= 1,
            "{label}"
        );
    }
    Ok(())
}

#[test]
fn another_screen_is_drawn_when_it_is_picked() -> TestResult {
    let mut window = opened(Language::BritishEnglish, WINDOW)?;
    pick_ship(&mut window)?;
    let first = window
        .harness
        .state()
        .ui()
        .rendered()
        .map(p4kx_gui::UiRendered::serial)
        .ok_or("no picture")?;

    window
        .harness
        .get_by_role_and_label(Role::Button, "Radar")
        .click();
    wait_until(&mut window.harness, |model| {
        drawn(model)
            && model
                .ui()
                .rendered()
                .is_some_and(|rendered| rendered.serial() != first)
    })?;

    let model = window.harness.state();
    assert_eq!(
        model.ui().ship().and_then(ShipPick::picked),
        Some(1),
        "the second screen"
    );
    let Some(UiSource::Screen { canvas, .. }) = model.ui().rendered().map(|r| r.source().clone())
    else {
        return Err("the picture is no screen".into());
    };
    assert!(canvas.ends_with("test_panel"), "{canvas}");
    Ok(())
}

#[test]
fn what_no_renderer_draws_is_reported_for_a_screen() -> TestResult {
    let mut window = opened(Language::BritishEnglish, WINDOW)?;
    pick_ship(&mut window)?;

    let model = window.harness.state();
    let report = model.ui().rendered().ok_or("no picture")?.report();
    assert!(
        report
            .lines()
            .iter()
            .any(|line| line.label == UiText::ReportLeftOut && line.detail.starts_with("WidgetImage")),
        "the picture widget: {:?}",
        report.lines()
    );
    let widgets = report
        .counts()
        .iter()
        .find(|(key, _)| *key == UiText::FactWidgets)
        .map(|(_, value)| *value);
    assert!(widgets.is_some_and(|widgets| widgets > 0), "{widgets:?}");
    let contents = model.ui().contents().ok_or("no contents")?;
    assert!(
        contents
            .rows()
            .iter()
            .any(|row| row.kind == UiText::KindElementUndrawn && row.detail.starts_with("WidgetImage")),
        "the screen's own widgets are listed with what is not drawn"
    );
    Ok(())
}

#[test]
fn the_texts_of_a_screen_follow_the_language_of_the_window() -> TestResult {
    let mut window = opened(Language::German, WINDOW)?;
    pick_ship(&mut window)?;

    assert_eq!(
        window.harness.state().record_text_language(),
        Some(p4kx_gui::TextLanguage::German),
        "German texts"
    );
    assert!(
        window.harness.query_all_by_label("BILDSCHIRME").count() >= 1,
        "the German heading"
    );
    Ok(())
}
