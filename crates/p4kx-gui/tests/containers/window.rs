//! Containers in the path tree of the window, driven headless.

use std::error::Error;
use std::time::{Duration, Instant};

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model, ui};
use tempfile::TempDir;

use crate::socpak::{LAYOUT, TestResult, archive};

fn window(language: Language) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    archive().write_to(&path)?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1600.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(language),
        );
    harness.state_mut().open(path, || {});
    settle(&mut harness, |harness| harness.state().browser().is_some())?;
    for folder in ["Data", "ObjectContainers"] {
        harness.get_by_role_and_label(Role::Button, folder).click();
        harness.step();
        harness.step();
    }
    Ok((dir, harness))
}

fn settle(
    harness: &mut Harness<'_, Model>,
    done: impl Fn(&Harness<'_, Model>) -> bool,
) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    harness.step();
    while !done(harness) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.step();
    Ok(())
}

#[test]
fn a_container_is_a_folder_that_can_be_expanded() -> TestResult {
    let (_dir, harness) = window(Language::BritishEnglish)?;

    assert!(
        harness
            .query_by_role_and_label(Role::Button, "room.socpak")
            .is_some(),
        "the container row"
    );
    assert!(
        harness.query_by_label("Expand room.socpak").is_some(),
        "its chevron"
    );
    Ok(())
}

#[test]
fn expanding_a_container_shows_its_folders() -> TestResult {
    let (_dir, mut harness) = window(Language::BritishEnglish)?;

    harness.get_by_label("Expand room.socpak").click();
    settle(&mut harness, |harness| {
        harness
            .query_by_role_and_label(Role::Button, "room")
            .is_some()
    })?;

    assert!(
        harness.query_by_label("Collapse room.socpak").is_some(),
        "expanded"
    );
    harness.get_by_label("Expand room").click();
    harness.step();
    harness.step();
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "lights.socpak")
            .is_some(),
        "the nested container"
    );
    Ok(())
}

#[test]
fn clicking_a_container_lists_and_previews_its_entries() -> TestResult {
    let (_dir, mut harness) = window(Language::BritishEnglish)?;

    harness
        .get_by_role_and_label(Role::Button, "room.socpak")
        .click();
    settle(&mut harness, |harness| {
        harness.query_by_label("layout.xml").is_some()
    })?;
    assert!(harness.query_by_label("chair.cgf").is_some(), "every Entry");
    assert!(harness.query_by_label("4 shown").is_some(), "status bar");
    harness.get_by_label("layout.xml").click();
    settle(&mut harness, |harness| {
        harness.query_by_label_contains("rooms=").is_some()
    })?;

    assert!(
        harness.query_by_label(LAYOUT.trim_end()).is_some(),
        "the preview reads it from the container"
    );
    Ok(())
}

#[test]
fn a_corrupt_container_says_so_on_its_row() -> TestResult {
    let (_dir, mut harness) = window(Language::BritishEnglish)?;

    harness
        .get_by_role_and_label(Role::Button, "broken.socpak")
        .click();
    settle(&mut harness, |harness| {
        harness.query_by_label("Cannot open").is_some()
    })?;

    assert!(
        harness.query_by_label("Expand broken.socpak").is_none(),
        "nothing to expand"
    );
    Ok(())
}

#[test]
fn the_notes_are_german_in_german() -> TestResult {
    let (_dir, mut harness) = window(Language::German)?;

    harness
        .get_by_role_and_label(Role::Button, "broken.socpak")
        .click();
    settle(&mut harness, |harness| {
        harness.query_by_label("Lässt sich nicht öffnen").is_some()
    })?;

    assert!(
        harness.query_by_label("room.socpak aufklappen").is_some(),
        "the other containers can still be expanded"
    );
    Ok(())
}
