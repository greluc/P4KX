//! The window driven headless through `egui_kittest`: open, navigate, sort, select.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model, SortColumn, SortDirection, ui};
use p4kx_testkit::{ArchiveBuilder, TestEntry};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Debug)]
struct Dropped(PathBuf);

impl egui::DroppedFile for Dropped {
    fn path(&self) -> &Path {
        &self.0
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|error| error.to_string())
    }
}

fn archive() -> Result<(TempDir, PathBuf), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Libs\\config.xml", vec![1; 20]))
        .entry(TestEntry::zstd("Data\\Objects\\ship.cgf", vec![2; 300]))
        .entry(TestEntry::stored("Data\\Objects\\anchor.cgf", vec![3; 10]).encrypted())
        .entry(TestEntry::stored("readme.txt", vec![4; 5]))
        .write_to(&path)?;
    Ok((folder, path))
}

fn harness(language: Language) -> Harness<'static, Model> {
    Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1200.0, 800.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(language),
        )
}

fn drop_and_wait(harness: &mut Harness<'_, Model>, path: &Path) -> TestResult {
    harness
        .input_mut()
        .dropped_files
        .push(Arc::new(Dropped(path.to_path_buf())));
    let deadline = Instant::now() + Duration::from_secs(10);
    while harness.state().browser().is_none() {
        if Instant::now() > deadline {
            return Err(format!("not opened: {:?}", harness.state().failure()).into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run();
    Ok(())
}

fn opened() -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let (folder, path) = archive()?;
    let mut harness = harness(Language::BritishEnglish);
    harness.run();
    drop_and_wait(&mut harness, &path)?;
    Ok((folder, harness))
}

fn row_order(harness: &Harness<'_, Model>, names: &[&str]) -> Vec<String> {
    let mut rows: Vec<(f32, String)> = names
        .iter()
        .filter_map(|&name| {
            harness
                .query_by_label(name)
                .map(|node| (node.rect().min.y, name.to_owned()))
        })
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows.into_iter().map(|(_, name)| name).collect()
}

#[test]
fn dropping_an_archive_opens_it_and_lists_every_entry() -> TestResult {
    let (_folder, harness) = opened()?;

    for name in ["anchor.cgf", "config.xml", "readme.txt", "ship.cgf"] {
        assert!(harness.query_by_label(name).is_some(), "{name} is listed");
    }
    assert!(
        harness.query_by_label("4 Entries").is_some(),
        "status bar count"
    );
    assert!(
        harness.query_by_label("Data.p4k").is_some(),
        "the root folder"
    );
    Ok(())
}

#[test]
fn the_window_starts_empty_with_a_hint() {
    let mut harness = harness(Language::BritishEnglish);

    harness.run();

    assert!(
        harness
            .query_by_label("Open an Archive or drop a Data.p4k onto the window.")
            .is_some(),
        "the empty status bar"
    );
}

#[test]
fn clicking_a_folder_shows_only_its_entries() -> TestResult {
    let (_folder, mut harness) = opened()?;

    harness.get_by_label("Data").click();
    harness.run();
    harness.get_by_label("Objects").click();
    harness.run();

    assert!(
        harness.query_by_label("ship.cgf").is_some(),
        "ship.cgf in Objects"
    );
    assert!(
        harness.query_by_label("config.xml").is_none(),
        "config.xml is in Libs"
    );
    assert!(harness.query_by_label("2 shown").is_some(), "status bar");
    harness.get_by_label("Collapse Data").click();
    harness.run();
    assert!(
        harness.query_by_label("Objects").is_none(),
        "Data collapsed"
    );
    Ok(())
}

#[test]
fn clicking_a_column_header_sorts_and_clicking_again_reverses() -> TestResult {
    let (_folder, mut harness) = opened()?;
    let names = ["anchor.cgf", "config.xml", "readme.txt", "ship.cgf"];

    harness.get_by_label("Size").click();
    harness.run();
    let ascending = row_order(&harness, &names);
    harness.get_by_label("Size").click();
    harness.run();

    assert_eq!(
        ascending,
        ["readme.txt", "anchor.cgf", "config.xml", "ship.cgf"],
        "smallest first"
    );
    assert_eq!(
        row_order(&harness, &names),
        ["ship.cgf", "config.xml", "anchor.cgf", "readme.txt"],
        "largest first"
    );
    let browser = harness.state().browser().ok_or("no Archive open")?;
    assert_eq!(browser.sort().column, SortColumn::Size, "sort column");
    assert_eq!(
        browser.sort().direction,
        SortDirection::Descending,
        "reversed"
    );
    Ok(())
}

#[test]
fn checkboxes_select_entries_one_by_one_or_all_shown() -> TestResult {
    let (_folder, mut harness) = opened()?;

    harness.get_by_label("Select ship.cgf").click();
    harness.run();
    let one = harness
        .state()
        .browser()
        .map(p4kx_gui::Browser::checked_count);
    harness.get_by_label("Select all shown Entries").click();
    harness.run();

    assert_eq!(one, Some(1), "one Entry checked");
    let browser = harness.state().browser().ok_or("no Archive open")?;
    assert_eq!(browser.checked_count(), 4, "every Entry checked");
    assert!(harness.query_by_label("4 selected").is_some(), "status bar");
    Ok(())
}

#[test]
fn the_language_menu_switches_every_text() -> TestResult {
    let (_folder, mut harness) = opened()?;

    harness.get_by_label("Language").click();
    harness.run();
    harness.get_by_label("Deutsch").click();
    harness.run();

    assert_eq!(
        harness.state().texts().language(),
        Language::German,
        "German is current"
    );
    assert!(
        harness.query_by_label("Größe").is_some(),
        "German column header"
    );
    assert!(
        harness.query_by_label("4 Einträge").is_some(),
        "German status bar"
    );
    Ok(())
}

#[test]
fn the_menus_ask_the_host_for_the_open_dialog_and_to_store_the_language() -> Result<(), &'static str>
{
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1200.0, 800.0])
        .build_ui_state(
            |ui, (model, requests): &mut (Model, ui::Requests)| {
                let frame = ui::show(ui, model);
                requests.dialog = requests.dialog.or(frame.dialog);
                requests.language_changed |= frame.language_changed;
            },
            (Model::new(Language::German), ui::Requests::default()),
        );

    harness.get_by_label("Archiv").click();
    harness.run();
    harness
        .get_all_by_label("Archiv öffnen …")
        .last()
        .ok_or("the menu entry is missing")?
        .click();
    harness.run();
    harness.get_by_label("Sprache").click();
    harness.run();
    harness.get_by_label("English (UK)").click();
    harness.run();

    let (model, requests) = harness.state();
    assert_eq!(
        requests.dialog,
        Some(ui::Dialog::OpenArchive),
        "the open dialog is requested"
    );
    assert!(requests.language_changed, "the language is to be stored");
    assert_eq!(
        model.texts().language(),
        Language::BritishEnglish,
        "switched"
    );
    Ok(())
}

#[test]
fn the_help_menu_shows_the_font_licence() {
    let mut harness = harness(Language::BritishEnglish);
    harness.run();

    harness.get_by_label("Help").click();
    harness.run();
    harness.get_by_label("Font licence").click();
    harness.run();

    assert!(harness.state().licence_shown(), "the licence is open");
    assert!(
        harness
            .query_by_label_contains("SIL OPEN FONT LICENSE Version 1.1")
            .is_some(),
        "the OFL text is shown"
    );
}

#[test]
fn the_help_menu_shows_the_legal_notices_in_both_languages() {
    for (language, help, entry, fan_project, responsibility) in [
        (
            Language::BritishEnglish,
            "Help",
            "About P4K-Extractor",
            "unofficial Star Citizen fan project",
            "Terms of Service",
        ),
        (
            Language::German,
            "Hilfe",
            "Über P4K-Extractor",
            "inoffizielles Star-Citizen-Fanprojekt",
            "Nutzungsbedingungen",
        ),
    ] {
        let mut harness = harness(language);
        harness.run();

        harness.get_by_label(help).click();
        harness.run();
        harness.get_by_label(entry).click();
        harness.run();

        assert!(harness.state().about_shown(), "{language:?}: About is open");
        for needle in [
            fan_project,
            responsibility,
            "Copyright (c) 2026 KRT and the P4K-Extractor contributors",
            "Star Citizen®, Roberts Space Industries® and Cloud Imperium® are registered trademarks of Cloud Imperium Rights LLC.",
        ] {
            assert!(
                harness.query_by_label_contains(needle).is_some(),
                "{language:?}: {needle}"
            );
        }
    }
}

#[test]
fn a_file_that_is_no_archive_shows_the_failure() -> TestResult {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("broken.p4k");
    std::fs::write(&path, b"not an archive")?;
    let mut harness = harness(Language::BritishEnglish);
    harness.run();
    harness
        .input_mut()
        .dropped_files
        .push(Arc::new(Dropped(path.clone())));
    let deadline = Instant::now() + Duration::from_secs(10);
    while harness.state().failure().is_none() {
        if Instant::now() > deadline {
            return Err("no failure reported".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run();

    let prefix = format!("Cannot open {}", path.display());
    assert!(
        harness.query_by_label_contains(&prefix).is_some(),
        "the status bar names the file"
    );
    Ok(())
}
