// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Opening a DataForge file on disk in place of an Archive, driven without drawing.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use p4kx_gui::{Browser, Language, Model, RecordTree, RecordTreeStatus};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct};
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
