// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Reload and Extract selected entries of the Archive menu and their shortcuts, driven headless.

use crate::waiting::{RunQuiet, wait_for};
use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model, ui};
use p4kx_testkit::{ArchiveBuilder, TestEntry};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

fn opened() -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path: PathBuf = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\config.xml", "config"))
        .entry(TestEntry::stored("readme.txt", "readme"))
        .write_to(&path)?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1400.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(Language::BritishEnglish),
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    Ok((dir, harness))
}

fn wait_until(harness: &mut Harness<'_, Model>, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(30);
    harness.step();
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run_quiet();
    Ok(())
}

fn click_menu_entry(harness: &mut Harness<'_, Model>, shortcut: &str) -> TestResult {
    wait_for(harness, "the menu entry to be shown once", |harness| {
        harness.query_all_by_label_contains(shortcut).count() == 1
    })?;
    harness.get_by_label_contains(shortcut).click();
    Ok(())
}

fn check_first(harness: &mut Harness<'_, Model>) -> TestResult {
    let browser = harness.state_mut().browser_mut().ok_or("no Archive")?;
    let id = *browser.shown().first().ok_or("no Entry")?;
    browser.set_checked(id, true);
    harness.run_quiet();
    Ok(())
}

fn checked(harness: &Harness<'_, Model>) -> usize {
    harness
        .state()
        .browser()
        .map_or(0, p4kx_gui::Browser::checked_count)
}

fn reopened(model: &Model) -> bool {
    model.opening_path().is_none() && model.browser().is_some_and(|b| b.checked_count() == 0)
}

#[test]
fn ctrl_e_opens_the_extraction_of_the_selected_entries() -> TestResult {
    let (_dir, mut harness) = opened()?;

    harness.key_press_modifiers(Modifiers::COMMAND, Key::E);
    harness.run_quiet();
    let without_selection = harness.state().extraction_setup().is_some();
    check_first(&mut harness)?;
    harness.key_press_modifiers(Modifiers::COMMAND, Key::E);
    harness.run_quiet();

    assert!(!without_selection, "nothing selected, nothing opens");
    assert!(
        harness.state().extraction_setup().is_some(),
        "the setup opens"
    );
    Ok(())
}

#[test]
fn the_menu_entry_extracts_the_selected_entries() -> TestResult {
    let (_dir, mut harness) = opened()?;
    check_first(&mut harness)?;

    harness.get_by_label("Archive").click();
    harness.run_quiet();
    click_menu_entry(&mut harness, "Ctrl+E")?;
    harness.run_quiet();

    assert!(
        harness.state().extraction_setup().is_some(),
        "the setup opens"
    );
    Ok(())
}

#[test]
fn f5_opens_the_archive_again() -> TestResult {
    let (_dir, mut harness) = opened()?;
    check_first(&mut harness)?;

    harness.key_press(Key::F5);
    wait_until(&mut harness, reopened)?;

    assert_eq!(checked(&harness), 0, "a fresh index");
    Ok(())
}

#[test]
fn the_menu_entry_opens_the_archive_again() -> TestResult {
    let (_dir, mut harness) = opened()?;
    check_first(&mut harness)?;

    harness.get_by_label("Archive").click();
    harness.run_quiet();
    click_menu_entry(&mut harness, "F5")?;
    wait_until(&mut harness, reopened)?;

    assert_eq!(checked(&harness), 0, "a fresh index");
    Ok(())
}

#[test]
fn f5_does_nothing_while_the_extraction_setup_is_open() -> TestResult {
    let (_dir, mut harness) = opened()?;
    check_first(&mut harness)?;
    harness.key_press_modifiers(Modifiers::COMMAND, Key::E);
    harness.run_quiet();

    harness.key_press(Key::F5);
    harness.run_quiet();

    assert!(
        harness.state().opening_path().is_none(),
        "the Archive is not opened again"
    );
    assert_eq!(checked(&harness), 1, "the selection stays");
    Ok(())
}
