// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The search field, the quick-filter chips and the status bar counters, driven headless.

use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model, ui};
use p4kx_testkit::{ArchiveBuilder, TestEntry};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

fn archive() -> Result<(TempDir, PathBuf), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Libs\\config.xml", vec![1; 20]))
        .entry(TestEntry::stored("Data\\Textures\\hull.dds", vec![2; 30]))
        .entry(TestEntry::stored("Data\\Game2.dcb", vec![3; 40]))
        .entry(TestEntry::stored(
            "Data\\Objects\\aurora\\seat.cgf",
            vec![4; 10],
        ))
        .write_to(&path)?;
    Ok((folder, path))
}

fn opened(language: Language) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let (folder, path) = archive()?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1200.0, 800.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(language),
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    Ok((folder, harness))
}

fn wait_until(harness: &mut Harness<'_, Model>, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    harness.step();
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run();
    Ok(())
}

fn searched(harness: &mut Harness<'_, Model>) -> TestResult {
    wait_until(harness, |model| {
        model
            .browser()
            .is_some_and(|browser| !browser.is_searching())
    })
}

#[test]
fn typing_into_the_search_field_filters_the_whole_archive() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;

    harness
        .get_by_role(egui::accesskit::Role::TextInput)
        .click();
    harness.run();
    harness
        .get_by_role(egui::accesskit::Role::TextInput)
        .type_text("HULL");
    searched(&mut harness)?;

    assert!(harness.query_by_label("hull.dds").is_some(), "the match");
    assert!(
        harness.query_by_label("config.xml").is_none(),
        "no other Entry"
    );
    assert!(harness.query_by_label("1 shown").is_some(), "status bar");
    Ok(())
}

#[test]
fn a_chip_applies_its_filter_and_a_second_click_clears_it() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;

    harness.get_by_label("*.dcb").click();
    searched(&mut harness)?;
    let with_chip = (
        harness.query_all_by_label("Game2.dcb").count(),
        harness.query_by_label("seat.cgf").is_some(),
    );
    harness.get_by_label("*.dcb").click();
    searched(&mut harness)?;

    assert_eq!(
        with_chip,
        (2, false),
        "the chip and only the DataForge Entry"
    );
    assert_eq!(
        harness
            .state()
            .browser()
            .map(p4kx_gui::Browser::search_text),
        Some(""),
        "cleared"
    );
    assert!(harness.query_by_label("4 shown").is_some(), "every Entry");
    Ok(())
}

#[test]
fn every_quick_filter_is_offered() -> TestResult {
    let (_folder, harness) = opened(Language::BritishEnglish)?;

    for chip in ["*.xml", "*.dds", "*.dcb", "Game2.dcb", "aurora"] {
        assert!(
            harness.query_all_by_label(chip).next().is_some(),
            "{chip} is offered"
        );
    }
    Ok(())
}

#[test]
fn the_status_bar_counts_selected_entries_outside_the_filter() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;
    harness.get_by_label("Select config.xml").click();
    harness.run();

    harness.get_by_label("aurora").click();
    searched(&mut harness)?;
    harness.get_by_label("Select seat.cgf").click();
    harness.run();

    assert!(harness.query_by_label("1 shown").is_some(), "shown");
    assert!(
        harness.query_by_label("2 selected, 1 not shown").is_some(),
        "selected, one outside the filter"
    );
    Ok(())
}

#[test]
fn the_german_status_bar_counts_hidden_selections_too() -> TestResult {
    let (_folder, mut harness) = opened(Language::German)?;
    harness.get_by_label("config.xml auswählen").click();
    harness.run();

    harness.get_by_label("*.dds").click();
    searched(&mut harness)?;

    assert!(
        harness
            .query_by_label("1 ausgewählt, davon 1 nicht angezeigt")
            .is_some(),
        "German counters"
    );
    Ok(())
}
