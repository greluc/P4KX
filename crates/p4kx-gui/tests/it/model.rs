// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The window state, driven without drawing: opening, navigating, sorting and checking.

use crate::waiting::wait_polling;
use std::cell::Cell;
use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_gui::{Browser, Language, Model, SortColumn, SortDirection};
use p4kx_testkit::{ArchiveBuilder, TestEntry};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

fn archive() -> Result<(TempDir, PathBuf), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Libs\\config.xml", vec![1; 20]))
        .entry(TestEntry::zstd("Data\\Objects\\ship.cgf", vec![2; 300]))
        .entry(TestEntry::stored("Data\\Objects\\anchor.cgf", vec![3; 10]))
        .entry(TestEntry::stored("readme.txt", vec![4; 5]))
        .write_to(&path)?;
    Ok((folder, path))
}

fn opened(path: &Path) -> Result<Model, Box<dyn Error>> {
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path.to_path_buf(), || {});
    let polled = Cell::new(false);
    wait_polling(
        &mut model,
        "the Archive to open",
        |model| polled.set(model.poll()),
        |_| polled.get(),
    )?;
    Ok(model)
}

fn shown_names(browser: &Browser) -> Vec<&str> {
    browser
        .shown()
        .iter()
        .map(|&id| browser.index().name(id))
        .collect()
}

#[test]
fn opening_shows_every_entry_sorted_by_name() -> TestResult {
    let (_folder, path) = archive()?;

    let model = opened(&path)?;

    let browser = model.browser().ok_or("no Archive open")?;
    assert_eq!(model.opening_path(), None, "nothing is opening any more");
    assert_eq!(
        shown_names(browser),
        ["anchor.cgf", "config.xml", "readme.txt", "ship.cgf"],
        "every Entry, by name"
    );
    Ok(())
}

#[test]
fn opening_a_missing_file_reports_the_failure() -> TestResult {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("missing.p4k");

    let model = opened(&path)?;

    let failure = model.failure().ok_or("no failure")?;
    assert_eq!(failure.path, path, "the failed path");
    assert!(
        failure
            .reason
            .as_deref()
            .is_some_and(|reason| !reason.is_empty()),
        "a reason"
    );
    assert!(model.browser().is_none(), "no Archive open");
    Ok(())
}

#[test]
fn selecting_a_folder_shows_its_entries_and_expands_it() -> TestResult {
    let (_folder, path) = archive()?;
    let mut model = opened(&path)?;
    let browser = model.browser_mut().ok_or("no Archive open")?;
    let data = browser.index().find_folder("Data").ok_or("no Data")?;
    let objects = browser
        .index()
        .find_folder("Data\\Objects")
        .ok_or("no Objects")?;

    browser.select_folder(data);
    let rows_after_data = browser.tree_rows().len();
    browser.select_folder(objects);

    assert_eq!(rows_after_data, 4, "root, Data, Libs, Objects");
    assert_eq!(browser.selected_folder(), objects, "selected");
    assert_eq!(
        shown_names(browser),
        ["anchor.cgf", "ship.cgf"],
        "Objects only"
    );
    browser.toggle_expanded(data);
    assert_eq!(browser.tree_rows().len(), 2, "root and the collapsed Data");
    Ok(())
}

#[test]
fn sorting_twice_by_a_column_reverses_it() -> TestResult {
    let (_folder, path) = archive()?;
    let mut model = opened(&path)?;
    let browser = model.browser_mut().ok_or("no Archive open")?;

    browser.sort_by(SortColumn::Size);
    let ascending = shown_names(browser).join(",");
    browser.sort_by(SortColumn::Size);

    assert_eq!(
        ascending, "readme.txt,anchor.cgf,config.xml,ship.cgf",
        "ascending"
    );
    assert_eq!(
        browser.sort().direction,
        SortDirection::Descending,
        "reversed"
    );
    assert_eq!(
        shown_names(browser),
        ["ship.cgf", "config.xml", "anchor.cgf", "readme.txt"],
        "descending"
    );
    Ok(())
}

#[test]
fn checking_counts_across_folders_and_all_shown_follows_the_folder() -> TestResult {
    let (_folder, path) = archive()?;
    let mut model = opened(&path)?;
    let browser = model.browser_mut().ok_or("no Archive open")?;
    let objects = browser
        .index()
        .find_folder("Data\\Objects")
        .ok_or("no Objects")?;

    browser.select_folder(objects);
    browser.set_all_shown_checked(true);
    let all_in_objects = browser.all_shown_checked();
    browser.select_folder(browser.index().root());

    assert!(all_in_objects, "every Entry of Objects is checked");
    assert_eq!(browser.checked_count(), 2, "two checked");
    assert!(
        !browser.all_shown_checked(),
        "the root shows unchecked Entries"
    );
    let first = *browser.shown().first().ok_or("nothing shown")?;
    browser.set_checked(first, false);
    assert_eq!(browser.checked_count(), 1, "anchor.cgf unchecked");
    Ok(())
}
