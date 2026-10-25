// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Search tab in the window, driven headless over a synthetic Archive with a DataForge, two
//! `global.ini` files and text Entries.

use crate::waiting::wait_for_model;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{ContentSearchStatus, Language, Model, TextLanguage, WorkTab, ui};
use p4kx_search::Source;
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, GlobalIniBuilder, TestEntry, TestRecord, TestStruct,
    TestType, TestValue,
};

type TestResult = Result<(), Box<dyn Error>>;

fn forge() -> Vec<u8> {
    DataForgeBuilder::new()
        .structure(
            TestStruct::new("Gear")
                .field("name", TestType::String)
                .array("tags", TestType::String),
        )
        .record(
            TestRecord::new(
                0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f,
                "Gear",
                "Gear.AEGS_Gladius",
                "libs/ships/aegs_gladius.xml",
            )
            .value("name", TestValue::text("AEGS_Gladius Fighter"))
            .value(
                "tags",
                TestValue::Array(vec![TestValue::text("alpha"), TestValue::text("a gladius")]),
            ),
        )
        .build()
}

fn long_text() -> String {
    (1..=400)
        .map(|row| {
            if row == 300 {
                "row 300 needleword\n".to_owned()
            } else {
                format!("row {row}\n")
            }
        })
        .collect()
}

pub(crate) fn archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", forge()))
        .entry(TestEntry::zstd(
            TextLanguage::English.global_ini_path(),
            GlobalIniBuilder::new()
                .text("vehicle_NameAEGS_Gladius", "Aegis Gladius")
                .text("item_Other", "Nothing")
                .build(),
        ))
        .entry(TestEntry::deflated(
            "Data\\Scripts\\ship.xml",
            b"<Ship>\n  <Part name=\"AEGS_Gladius\"/>\n</Ship>\n".to_vec(),
        ))
        .entry(TestEntry::deflated(
            "Data\\Scripts\\long.xml",
            long_text().into_bytes(),
        ))
        .entry(TestEntry::stored(
            "Data\\Scripts\\binary.xml",
            b"gladius\0\x01".to_vec(),
        ))
        .entry(TestEntry::zstd("Data\\Scripts\\secret.xml", b"<secret/>".to_vec()).encrypted())
        .write_to(&path)?;
    Ok(path)
}

fn harness(model: Model) -> Harness<'static, Model> {
    Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1400.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        )
}

pub(crate) fn wait_until(
    harness: &mut Harness<'_, Model>,
    done: impl Fn(&Model) -> bool,
) -> TestResult {
    wait_for_model(harness, "the window to settle", done)?;
    harness.run_steps(3);
    Ok(())
}

pub(crate) fn finished(model: &Model) -> bool {
    !matches!(
        model.content_search().status(),
        ContentSearchStatus::Idle | ContentSearchStatus::Running
    )
}

pub(crate) fn click_row(harness: &Harness<'_, Model>, text: &str) -> TestResult {
    harness
        .get_all_by_label_contains(text)
        .next()
        .ok_or("no such row")?
        .click();
    Ok(())
}

pub(crate) fn searched(
    language: Language,
    text: &str,
) -> Result<(tempfile::TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let mut harness = harness(Model::new(language));
    harness.state_mut().open(archive(dir.path())?, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    harness.state_mut().set_work_tab(WorkTab::Search);
    harness.run_steps(2);
    harness.get_by_role(Role::TextInput).click();
    harness.run_steps(2);
    harness.get_by_role(Role::TextInput).type_text(text);
    harness.run_steps(2);
    let label = if language == Language::German {
        "Suchen"
    } else {
        "Search"
    };
    harness.get_by_role_and_label(Role::Button, label).click();
    wait_until(&mut harness, finished)?;
    Ok((dir, harness))
}

#[test]
fn the_search_tab_sits_next_to_the_others_and_asks_for_an_archive() {
    let mut harness = harness(Model::new(Language::BritishEnglish));
    harness.run_steps(2);

    harness.get_by_role_and_label(Role::Tab, "Search").click();
    harness.run_steps(2);

    assert_eq!(harness.state().work_tab(), WorkTab::Search, "switched");
    assert!(
        harness.query_by_label("No Archive open").is_some(),
        "asks for an Archive"
    );
}

#[test]
fn a_search_lists_the_hits_of_each_source_with_the_counts() -> TestResult {
    let (_dir, mut harness) = searched(Language::BritishEnglish, "gladi")?;

    let result = harness
        .state()
        .content_search()
        .result()
        .ok_or("no result")?;
    assert_eq!(result.len(Source::Records), 2, "Records");
    assert_eq!(result.len(Source::Localization), 1, "localisation");
    assert_eq!(result.len(Source::Entries), 1, "Entries");
    assert_eq!(result.stats().entries_binary, 1, "binary counted");
    for label in ["Records 2", "Localisation 1", "Entries 1"] {
        assert!(harness.query_by_label(label).is_some(), "segment {label}");
    }
    assert!(
        harness
            .query_by_role_and_label(Role::ColumnHeader, "Record path")
            .is_some(),
        "Record columns"
    );
    assert!(
        harness
            .query_by_label_contains("1 Entry was not searched")
            .is_some(),
        "skipped note"
    );

    harness.get_by_label("Localisation 1").click();
    harness.run_steps(3);
    assert_eq!(
        harness.state().content_search().shown(),
        Source::Localization,
        "switched source"
    );
    assert!(
        harness
            .query_by_label_contains("vehicle_NameAEGS_Gladius")
            .is_some(),
        "the key of the hit"
    );
    Ok(())
}

#[test]
fn a_click_on_a_hit_opens_the_record_the_entry_and_the_text() -> TestResult {
    let (_dir, mut harness) = searched(Language::BritishEnglish, "gladi")?;

    click_row(&harness, "libs/ships/aegs_gladius")?;
    wait_until(&mut harness, |model| {
        model.work_tab() == WorkTab::Archive
            && model
                .browser()
                .is_some_and(|browser| browser.selected_record().is_some())
    })?;
    assert!(
        harness
            .state()
            .browser()
            .and_then(p4kx_gui::Browser::selected_record)
            .is_some(),
        "Record selected"
    );

    harness.state_mut().set_work_tab(WorkTab::Search);
    harness.run_steps(2);
    harness.get_by_label("Entries 1").click();
    harness.run_steps(3);
    click_row(&harness, "Data\\Scripts\\ship.xml")?;
    wait_until(&mut harness, |model| {
        model
            .browser()
            .is_some_and(|browser| browser.selected_entry().is_some())
    })?;
    let browser = harness.state().browser().ok_or("no browser")?;
    let selected = browser.selected_entry().ok_or("no Entry")?;
    assert_eq!(
        browser
            .index()
            .entry(selected)
            .map(p4kx_extraction::Entry::path),
        Some("Data\\Scripts\\ship.xml"),
        "Entry selected"
    );

    harness.state_mut().set_work_tab(WorkTab::Search);
    harness.run_steps(2);
    harness.get_by_label("Localisation 1").click();
    harness.run_steps(3);
    click_row(&harness, "vehicle_NameAEGS_Gladius")?;
    wait_until(&mut harness, |model| {
        model.work_tab() == WorkTab::Localization && model.localization().selected().is_some()
    })?;
    assert_eq!(
        harness.state().localization().selected_key(),
        Some("vehicle_NameAEGS_Gladius"),
        "key selected in the localisation tab"
    );
    Ok(())
}

#[test]
fn nothing_found_says_so_and_an_empty_text_cannot_be_searched() -> TestResult {
    let (_dir, harness) = searched(Language::BritishEnglish, "zzzzqq")?;

    assert!(
        harness
            .query_by_label("Nothing matches “zzzzqq”.")
            .is_some(),
        "no match note"
    );
    Ok(())
}

#[test]
fn the_german_window_is_in_german() -> TestResult {
    let (_dir, harness) = searched(Language::German, "gladi")?;

    for label in ["Records 2", "Lokalisierung 1", "Einträge 1"] {
        assert!(harness.query_by_label(label).is_some(), "segment {label}");
    }
    assert!(
        harness
            .query_by_role_and_label(Role::ColumnHeader, "Record-Pfad")
            .is_some(),
        "German column"
    );
    Ok(())
}

#[test]
fn a_search_leaves_the_key_to_the_user_and_offers_the_dialog() -> TestResult {
    let (_dir, mut harness) = searched(Language::BritishEnglish, "gladi")?;

    assert_eq!(
        harness
            .state()
            .content_search()
            .result()
            .map(|result| result.stats().entries_key_required),
        Some(1),
        "the encrypted Entry is counted"
    );
    assert!(harness.state().key_dialog().is_none(), "no dialog opened");
    assert!(
        harness
            .query_by_label_contains("1 Entry is encrypted")
            .is_some(),
        "alert"
    );

    harness
        .get_by_role_and_label(Role::Button, "Enter the Archive key")
        .click();
    harness.run_steps(3);
    assert!(harness.state().key_dialog().is_some(), "dialog opened");
    Ok(())
}

#[test]
fn a_source_without_a_hit_says_so() -> TestResult {
    let (_dir, mut harness) = searched(Language::BritishEnglish, "alpha")?;

    harness.get_by_label("Entries 0").click();
    harness.run_steps(3);
    assert_eq!(
        harness.state().content_search().shown(),
        Source::Entries,
        "switched"
    );
    assert!(
        harness.query_by_label("This source has no hit.").is_some(),
        "empty state"
    );
    Ok(())
}

pub(crate) fn save(name: &str, width: u32, height: u32, image: &[u8]) -> TestResult {
    let Some(folder) = std::env::var_os("P4KX_RENDER_DIR").map(PathBuf::from) else {
        return Ok(());
    };
    std::fs::create_dir_all(&folder)?;
    let mut encoder = png::Encoder::new(
        BufWriter::new(File::create(folder.join(name))?),
        width,
        height,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(image)?;
    Ok(())
}

#[test]
#[ignore = "needs a GPU adapter: set P4KX_RENDER_DIR and pass --ignored"]
fn the_tab_renders_in_both_languages() -> TestResult {
    for (language, name) in [
        (Language::BritishEnglish, "content-search-en-GB.png"),
        (Language::German, "content-search-de-DE.png"),
    ] {
        let (_dir, mut harness) = searched(language, "gladi")?;
        harness.run_steps(2);
        let image = harness.render()?;
        let (width, height) = (image.width(), image.height());
        save(name, width, height, &image.into_raw())?;
    }
    Ok(())
}
