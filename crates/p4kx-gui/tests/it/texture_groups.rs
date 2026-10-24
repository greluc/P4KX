// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Split textures grouped in the Entry table: one row per texture, parts reachable, and checks,
//! search and Extraction that cover all parts.

use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{
    Browser, EntryRowKind, ExtractionScope, ExtractionStatus, Language, Model, Sort, SortColumn,
    SortDirection, TargetKind, ui,
};
use p4kx_testkit::{ArchiveBuilder, TestEntry};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

fn archive() -> Result<(TempDir, PathBuf), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\a.txt", vec![1; 5]))
        .entry(TestEntry::stored("Data\\tex\\lonely.dds", vec![2; 50]))
        .entry(TestEntry::stored("Data\\tex\\orphan.dds.1", vec![3; 7]))
        .entry(TestEntry::stored("Data\\tex\\wall.dds", vec![4; 100]))
        .entry(TestEntry::stored("Data\\tex\\wall.dds.1", vec![5; 20]))
        .entry(TestEntry::stored("Data\\tex\\wall.dds.2", vec![6; 3]))
        .entry(TestEntry::stored("Data\\tex\\wall.dds.a", vec![7; 4]))
        .write_to(&path)?;
    Ok((folder, path))
}

fn settle(model: &mut Model, mut done: impl FnMut(&mut Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(model) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
    }
    Ok(())
}

fn opened() -> Result<(TempDir, Model), Box<dyn Error>> {
    let (folder, path) = archive()?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path, || {});
    settle(&mut model, |model| model.browser().is_some())?;
    Ok((folder, model))
}

fn browser(model: &mut Model) -> Result<&mut Browser, Box<dyn Error>> {
    Ok(model.browser_mut().ok_or("no Archive")?)
}

fn id_of(browser: &Browser, path: &str) -> Result<u32, Box<dyn Error>> {
    let index = browser.index();
    (0..u32::try_from(index.len())?)
        .find(|&id| index.entry(id).is_some_and(|entry| entry.path() == path))
        .ok_or_else(|| format!("no {path}").into())
}

fn names(browser: &Browser) -> Vec<String> {
    browser
        .rows()
        .iter()
        .map(|row| browser.index().name(row.id).to_owned())
        .collect()
}

#[test]
fn a_texture_is_one_row_and_its_parts_expand_below_it() -> TestResult {
    let (_folder, mut model) = opened()?;
    let browser = browser(&mut model)?;

    assert_eq!(
        names(browser),
        ["a.txt", "lonely.dds", "orphan.dds.1", "wall.dds"],
        "one row per texture; the orphaned part and the lone header stay visible"
    );
    let wall = id_of(browser, "Data\\tex\\wall.dds")?;
    let kinds: Vec<_> = browser.rows().iter().map(|row| row.kind).collect();
    assert_eq!(
        kinds.last(),
        Some(&EntryRowKind::Texture { parts: 3 }),
        "the texture counts its parts"
    );
    assert_eq!(browser.index().texture_size(wall), Some(127), "total size");

    browser.toggle_texture(wall);

    assert_eq!(
        names(browser),
        [
            "a.txt",
            "lonely.dds",
            "orphan.dds.1",
            "wall.dds",
            "wall.dds.1",
            "wall.dds.2",
            "wall.dds.a"
        ],
        "the parts below the header"
    );
    assert_eq!(
        browser.rows().last().map(|row| row.kind),
        Some(EntryRowKind::Part)
    );
    browser.toggle_texture(wall);
    assert_eq!(browser.rows().len(), 4, "collapsed again");
    Ok(())
}

#[test]
fn checking_a_texture_checks_all_its_parts() -> TestResult {
    let (_folder, mut model) = opened()?;
    let browser = browser(&mut model)?;
    let wall = id_of(browser, "Data\\tex\\wall.dds")?;
    let part = id_of(browser, "Data\\tex\\wall.dds.2")?;

    browser.set_checked(wall, true);

    assert_eq!(browser.checked_count(), 4, "header and three parts");
    assert!(browser.is_checked(part), "a part");
    assert_eq!(browser.scope_len(ExtractionScope::Selected), 4);
    browser.set_checked(part, false);
    assert_eq!(browser.checked_count(), 0, "unchecking a part unchecks all");

    browser.set_all_shown_checked(true);
    assert_eq!(
        browser.checked_count(),
        browser.index().len(),
        "every Entry"
    );
    assert!(browser.all_shown_checked());
    assert_eq!(
        browser.scope_len(ExtractionScope::Shown),
        7,
        "the parts count as shown"
    );
    assert_eq!(browser.scope_entries(ExtractionScope::Shown).len(), 7);
    Ok(())
}

#[test]
fn a_search_matching_one_part_shows_the_texture_with_all_parts() -> TestResult {
    let (_folder, mut model) = opened()?;
    browser(&mut model)?.search("wall.dds.2", || {});
    settle(&mut model, |model| {
        model.browser_mut().is_some_and(|browser| {
            browser.poll();
            !browser.is_searching()
        })
    })?;
    let browser = browser(&mut model)?;

    assert_eq!(names(browser), ["wall.dds"], "one row for the texture");
    assert_eq!(
        browser.scope_len(ExtractionScope::Shown),
        4,
        "Extraction of the shown Entries covers all parts"
    );
    browser.set_all_shown_checked(true);
    assert_eq!(browser.checked_count(), 4);
    assert!(browser.all_shown_checked());
    Ok(())
}

#[test]
fn sorting_by_size_uses_the_size_of_the_whole_texture() -> TestResult {
    let (_folder, mut model) = opened()?;
    let browser = browser(&mut model)?;

    browser.sort_by(SortColumn::Size);
    let ascending = names(browser);
    browser.sort_by(SortColumn::Size);
    let descending = names(browser);

    assert_eq!(
        ascending,
        ["a.txt", "orphan.dds.1", "lonely.dds", "wall.dds"],
        "127 bytes in all is the largest"
    );
    assert_eq!(descending.first().map(String::as_str), Some("wall.dds"));
    assert_eq!(
        browser.sort(),
        Sort {
            column: SortColumn::Size,
            direction: SortDirection::Descending
        }
    );
    Ok(())
}

#[test]
fn extracting_a_checked_texture_writes_all_its_parts() -> TestResult {
    let (folder, mut model) = opened()?;
    let wall = id_of(browser(&mut model)?, "Data\\tex\\wall.dds")?;
    browser(&mut model)?.set_checked(wall, true);
    let out = folder.path().join("out");
    std::fs::create_dir(&out)?;

    model.begin_extraction(ExtractionScope::Selected);
    let setup = model
        .extraction_setup_mut()
        .ok_or("the Extraction setup did not open")?;
    setup.set_kind(TargetKind::ExistingFolder);
    setup.set_folder(out.clone());
    assert!(model.submit_extraction(|| {}), "started");
    settle(&mut model, |model| {
        model.poll_extraction();
        model
            .extraction()
            .is_some_and(|extraction| matches!(extraction.status(), ExtractionStatus::Finished(_)))
    })?;

    for name in ["wall.dds", "wall.dds.1", "wall.dds.2", "wall.dds.a"] {
        assert!(out.join("Data").join("tex").join(name).is_file(), "{name}");
    }
    assert!(!out.join("Data").join("a.txt").exists(), "nothing else");
    Ok(())
}

#[test]
fn the_window_shows_one_row_and_expands_the_parts_on_the_chevron() -> TestResult {
    let (_folder, path) = archive()?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1200.0, 800.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(Language::BritishEnglish),
        );
    harness.state_mut().open(path, || {});
    let deadline = Instant::now() + Duration::from_secs(10);
    while harness.state().browser().is_none() {
        assert!(Instant::now() < deadline, "the Archive did not open");
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run();
    let wall = harness
        .state()
        .browser()
        .map(|browser| id_of(browser, "Data\\tex\\wall.dds"))
        .ok_or("no Archive")??;
    let folder = harness
        .state()
        .browser()
        .and_then(|browser| browser.index().find_folder("Data\\tex"))
        .ok_or("no folder")?;
    harness
        .state_mut()
        .browser_mut()
        .ok_or("no Archive")?
        .select_folder(folder);
    harness.run();

    assert!(harness.query_by_label("wall.dds").is_some(), "the texture");
    assert!(
        harness.query_by_label("wall.dds.1").is_none(),
        "no part yet"
    );

    harness.get_by_label("Show the parts of wall.dds").click();
    harness.run();

    assert!(harness.query_by_label("wall.dds.1").is_some(), "a part");
    assert!(harness.query_by_label("wall.dds.a").is_some(), "the tail");
    assert!(
        harness
            .state()
            .browser()
            .is_some_and(|browser| browser.is_texture_expanded(wall)),
        "expanded"
    );
    harness.get_by_label("Hide the parts of wall.dds").click();
    harness.run();
    assert!(harness.query_by_label("wall.dds.1").is_none(), "collapsed");
    Ok(())
}
