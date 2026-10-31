// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Opening a DataForge file on disk in place of an Archive, driven without drawing.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{
    Browser, ExtractionScope, Language, Model, NoEntries, PreviewTab, RecordTree, RecordTreeStatus,
    WorkTab, ui,
};
use p4kx_testkit::{ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct};
use tempfile::TempDir;

use crate::waiting::wait_polling;

type TestResult = Result<(), Box<dyn Error>>;

fn database() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(TestStruct::new("Vehicle"))
        .record(TestRecord::new(
            1,
            "Vehicle",
            "Vehicle.Cart",
            "libs/foundry/records/cart.xml",
        ))
        .record(TestRecord::new(
            2,
            "Vehicle",
            "Vehicle.Truck",
            "libs/foundry/records/truck.xml",
        ))
}

fn opened(name: &str, bytes: &[u8]) -> Result<(TempDir, PathBuf, Model), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join(name);
    fs::write(&path, bytes)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path.clone(), || {});
    wait_until(&mut model, |model| {
        model.browser().is_some() || model.failure().is_some()
    })?;
    Ok((dir, path, model))
}

fn wait_until(model: &mut Model, done: impl Fn(&Model) -> bool) -> TestResult {
    wait_polling(
        model,
        "the window to settle",
        |model| {
            model.poll();
            if let Some(browser) = model.browser_mut() {
                browser.poll();
            }
        },
        done,
    )?;
    Ok(())
}

fn records(model: &Model) -> Result<&RecordTree, Box<dyn Error>> {
    model
        .browser()
        .and_then(Browser::records)
        .ok_or_else(|| "no DataForge node".into())
}

#[test]
fn the_tabs_that_read_entries_and_the_catalog_are_off_for_a_file() -> TestResult {
    let (_dir, _, mut model) = opened("Game2.dcb", &database().build())?;
    model.set_work_tab(WorkTab::Sounds);

    assert!(model.is_dataforge_file());
    assert!(!model.catalog_available(), "the catalog reads Entries");
    for tab in [
        WorkTab::Localization,
        WorkTab::Search,
        WorkTab::Entities,
        WorkTab::Sounds,
        WorkTab::Characters,
        WorkTab::Ui,
    ] {
        assert!(tab.needs_entries(), "{tab:?}");
    }
    for tab in [WorkTab::Archive, WorkTab::Query, WorkTab::Tags] {
        assert!(!tab.needs_entries(), "{tab:?}");
    }
    Ok(())
}

#[test]
fn a_file_has_no_archive_so_no_job_that_reads_entries_can_run() -> TestResult {
    let (_dir, path, model) = opened("Game2.dcb", &database().build())?;
    let browser = model.browser().ok_or("nothing is open")?;
    let index = browser.archive_index();

    assert!(index.archive().is_none(), "no Archive stands in for it");
    assert_eq!(index.entries_archive().err(), Some(NoEntries));
    assert_eq!(index.path(), path.as_path());
    assert!(index.build_of_archive().is_none());
    assert_eq!(model.scope_len(ExtractionScope::All), 0);
    assert_eq!(index.dataforge_name().as_deref(), Some("Game2.dcb"));
    assert_eq!(index.dataforge_size(), fs::metadata(&path)?.len());
    Ok(())
}

#[test]
fn a_file_shows_only_its_dataforge_node() -> TestResult {
    let (_dir, _, model) = opened("Game2.dcb", &database().build())?;
    let browser = model.browser().ok_or("nothing is open")?;

    assert!(browser.tree_rows().is_empty(), "no folder of Entries");
    assert!(browser.shows_records(), "the list is the Record search");
    assert_eq!(model.preview_tab(), PreviewTab::DataForge);
    Ok(())
}

fn window(model: Model) -> Harness<'static, Model> {
    Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1400.0, 800.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        )
}

#[test]
fn the_window_of_a_file_has_no_entry_status_and_that_of_an_archive_has() -> TestResult {
    let (_dir, _, file) = opened("Game2.dcb", &database().build())?;
    let mut file = window(file);
    file.run_steps(3);

    assert!(file.query_by_label_contains("Build unknown").is_none());
    assert!(file.query_by_label("0 Entries").is_none());

    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.txt", "a"))
        .write_to(&path)?;
    let mut archive = Model::new(Language::BritishEnglish);
    archive.open(path, || {});
    wait_until(&mut archive, |model| model.browser().is_some())?;
    let mut archive = window(archive);
    archive.run_steps(3);

    archive.get_by_label_contains("Build unknown");
    archive.get_by_label("1 Entry");
    Ok(())
}

#[test]
fn opening_a_file_leaves_a_tab_that_reads_entries() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Game2.dcb");
    fs::write(&path, database().build())?;
    let mut model = Model::new(Language::BritishEnglish);
    model.set_work_tab(WorkTab::Sounds);

    model.open(path, || {});
    wait_until(&mut model, |model| model.browser().is_some())?;

    assert_eq!(model.work_tab(), WorkTab::Archive);
    Ok(())
}

#[test]
fn a_dataforge_file_opens_with_its_node_and_no_entries() -> TestResult {
    let (_dir, _, model) = opened("kept.bin", &database().build())?;

    let browser = model.browser().ok_or("nothing is open")?;

    assert_eq!(records(&model)?.entry_path(), "kept.bin", "named by file");
    assert!(browser.archive_index().is_dataforge_file());
    assert!(browser.archive_index().is_empty(), "a file has no Entries");
    Ok(())
}

#[test]
fn the_records_of_a_file_load_from_the_file() -> TestResult {
    let (_dir, _, mut model) = opened("Game2.dcb", &database().build())?;
    let root = records(&model)?.root();

    model
        .browser_mut()
        .ok_or("nothing is open")?
        .toggle_record_node(root, || {});
    wait_until(&mut model, |model| {
        records(model).is_ok_and(|tree| !matches!(tree.status(), RecordTreeStatus::Loading))
    })?;

    let RecordTreeStatus::Ready(summary) = records(&model)?.status() else {
        return Err("the DataForge did not load".into());
    };
    assert_eq!(summary.record_count(), 2);
    Ok(())
}

#[test]
fn a_file_of_an_unsupported_version_shows_the_failure_in_the_node() -> TestResult {
    let (_dir, _, mut model) = opened("old.dcb", &database().version(5).build())?;
    let root = records(&model)?.root();

    model
        .browser_mut()
        .ok_or("nothing is open")?
        .toggle_record_node(root, || {});
    wait_until(&mut model, |model| {
        records(model).is_ok_and(|tree| !matches!(tree.status(), RecordTreeStatus::Loading))
    })?;

    assert!(matches!(
        records(&model)?.status(),
        RecordTreeStatus::Failed(_)
    ));
    Ok(())
}

#[test]
fn a_file_that_is_neither_fails_to_open() -> TestResult {
    let (_dir, _, model) = opened("notes.txt", b"just some text, not a database")?;

    assert!(model.browser().is_none());
    assert!(model.failure().is_some());
    Ok(())
}
