// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The DataForge node in the path tree and the DataForge tab of the preview, driven headless.

use crate::waiting::RunQuiet;
use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Browser, Language, Model, PreviewTab, RecordTreeStatus, SUMMARY_RECORDS, ui};
use p4kx_testkit::{ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

fn database() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .version(8)
        .structure(TestStruct::new("EntityClassDefinition"))
        .structure(TestStruct::new("AmmoParams"))
        .enumeration("ItemType", ["Ship"])
        .record(TestRecord::new(
            0x11,
            "EntityClassDefinition",
            "EntityClassDefinition.AEGS_Avenger",
            "libs/foundry/records/entities/aegs_avenger.xml",
        ))
        .record(TestRecord::new(
            0x12,
            "AmmoParams",
            "AmmoParams.Laser_S1",
            "libs/foundry/records/ammo.xml",
        ))
        .record(TestRecord::new(
            0x13,
            "AmmoParams",
            "AmmoParams.Ballistic_S1",
            "libs/foundry/records/ammo.xml",
        ))
        .record(TestRecord::new(
            0x14,
            "EntityClassDefinition",
            "EntityClassDefinition.ANVL_Arrow",
            "libs/foundry/records/entities/anvl_arrow.xml",
        ))
}

fn large() -> DataForgeBuilder {
    (0..120_000_u128).fold(
        DataForgeBuilder::new().structure(TestStruct::new("Item")),
        |builder, n| {
            builder.record(TestRecord::new(
                n + 1,
                "Item",
                format!("Item.N{n}"),
                format!("libs/items/group{:03}/item{n:06}.xml", n % 600),
            ))
        },
    )
}

fn archive_with(bytes: Vec<u8>) -> ArchiveBuilder {
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Libs\\config.xml", "config"))
        .entry(TestEntry::zstd("Data\\Game2.dcb", bytes))
        .entry(TestEntry::stored("readme.txt", "readme"))
}

fn window(
    builder: &ArchiveBuilder,
    language: Language,
) -> Result<(TempDir, PathBuf, Harness<'static, Model>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    builder.write_to(&path)?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1600.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(language),
        );
    harness.state_mut().open(path.clone(), || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    Ok((dir, path, harness))
}

fn wait_until(harness: &mut Harness<'_, Model>, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(60);
    harness.step();
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.step();
    Ok(())
}

fn status_is(model: &Model, want: fn(&RecordTreeStatus<'_>) -> bool) -> bool {
    model
        .browser()
        .and_then(Browser::records)
        .is_some_and(|tree| want(&tree.status()))
}

fn loading(model: &Model) -> bool {
    status_is(model, |status| matches!(status, RecordTreeStatus::Loading))
}

fn settled(model: &Model) -> bool {
    status_is(model, |status| {
        !matches!(
            status,
            RecordTreeStatus::Loading | RecordTreeStatus::NotLoaded
        )
    })
}

#[test]
fn the_node_follows_the_folders_of_the_archive() -> TestResult {
    let (_dir, _, harness) = window(&archive_with(database().build()), Language::BritishEnglish)?;

    assert!(
        harness
            .query_by_role_and_label(Role::Button, "DataForge")
            .is_some(),
        "the node"
    );
    assert!(
        harness.query_by_label("Expand DataForge").is_some(),
        "it can be expanded"
    );
    Ok(())
}

#[test]
fn expanding_shows_a_loading_indicator_while_the_window_keeps_working() -> TestResult {
    let (_dir, _, mut harness) = window(&archive_with(large().build()), Language::BritishEnglish)?;

    harness.get_by_label("Expand DataForge").click();
    harness.step();
    harness.step();
    let indicator = harness.query_by_label("Loading…").is_some();
    harness.get_by_role_and_label(Role::Button, "Data").click();
    harness.step();
    let browsing = harness.state().browser().map(Browser::selected_folder);
    let still_loading = loading(harness.state());
    wait_until(&mut harness, settled)?;

    assert!(indicator, "the node shows that it loads");
    assert!(still_loading, "the click was handled while loading");
    let data = harness
        .state()
        .browser()
        .and_then(|browser| browser.index().find_folder("Data"));
    assert_eq!(browsing, data, "a folder was selected meanwhile");
    assert!(harness.query_by_label("libs").is_some(), "the Records");
    Ok(())
}

#[test]
fn an_unknown_version_is_named_on_the_node_and_browsing_goes_on() -> TestResult {
    let bytes = database().version(9).build();
    let (_dir, _, mut harness) = window(&archive_with(bytes), Language::BritishEnglish)?;

    harness.get_by_label("Expand DataForge").click();
    wait_until(&mut harness, settled)?;
    harness.get_by_role_and_label(Role::Button, "Data").click();
    harness.run_quiet();

    assert!(
        harness
            .query_by_label_contains("DataForge version 9 is not supported")
            .is_some(),
        "the message"
    );
    assert!(harness.query_by_label("config.xml").is_some(), "the table");
    Ok(())
}

#[test]
fn the_dataforge_tab_shows_version_counts_and_first_records() -> TestResult {
    let (_dir, _, mut harness) =
        window(&archive_with(database().build()), Language::BritishEnglish)?;

    harness
        .get_by_role_and_label(Role::Button, "DataForge")
        .click();
    wait_until(&mut harness, settled)?;

    assert_eq!(
        harness.state().preview_tab(),
        PreviewTab::DataForge,
        "the tab opens"
    );
    for (key, value) in [
        ("Version", "8"),
        ("Records", "4"),
        ("Structs", "2"),
        ("Enums", "1"),
    ] {
        assert!(harness.query_by_label(key).is_some(), "{key}");
        assert!(harness.query_by_label(value).is_some(), "{key} {value}");
    }
    for label in [
        "First Records",
        "EntityClassDefinition.AEGS_Avenger",
        "AmmoParams.Ballistic_S1",
        "libs/foundry/records/entities/anvl_arrow",
    ] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    Ok(())
}

#[test]
fn selecting_an_entry_returns_from_the_dataforge_tab_to_its_preview() -> TestResult {
    let (_dir, _, mut harness) =
        window(&archive_with(database().build()), Language::BritishEnglish)?;
    harness
        .get_by_role_and_label(Role::Button, "DataForge")
        .click();
    wait_until(&mut harness, settled)?;
    harness
        .get_by_role_and_label(Role::Button, "Data.p4k")
        .click();
    harness.run_quiet();

    harness.get_by_label("readme.txt").click();
    wait_until(&mut harness, |model| {
        model
            .browser()
            .and_then(Browser::preview)
            .is_some_and(|preview| !preview.is_loading())
    })?;

    assert_eq!(
        harness.state().preview_tab(),
        PreviewTab::Text,
        "the Text tab"
    );
    assert!(harness.query_by_label("readme").is_some(), "the Entry");
    Ok(())
}

#[test]
fn the_german_tab_names_the_first_records() -> TestResult {
    let (_dir, _, mut harness) = window(&archive_with(database().build()), Language::German)?;

    harness
        .get_by_role_and_label(Role::Tab, "DataForge")
        .click();
    wait_until(&mut harness, settled)?;

    assert!(harness.query_by_label("Erste Records").is_some(), "heading");
    assert!(harness.query_by_label("Record-Pfad").is_some(), "column");
    Ok(())
}

#[test]
fn a_change_on_disk_takes_the_records_out_of_the_tree() -> TestResult {
    let (_dir, path, mut harness) =
        window(&archive_with(database().build()), Language::BritishEnglish)?;
    harness.get_by_label("Expand DataForge").click();
    wait_until(&mut harness, settled)?;
    if harness.query_by_label("libs").is_none() {
        return Err("not loaded".into());
    }

    archive_with(large().build()).write_to(&path)?;
    harness.state_mut().check_archive();
    harness.run_quiet();

    assert!(harness.query_by_label("libs").is_none(), "dropped");
    assert!(
        harness
            .query_by_label_contains("Reload it to load DataForge")
            .is_some(),
        "the node says why"
    );
    Ok(())
}

#[test]
fn the_user_help_states_how_many_records_the_tab_lists() -> TestResult {
    let help = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/user-help.md"
    ))?;

    assert!(
        help.contains(&format!("the first {SUMMARY_RECORDS} Records")),
        "the number of Records listed"
    );
    Ok(())
}

#[test]
fn a_hundred_and_twenty_thousand_records_stay_virtual() -> TestResult {
    let (_dir, _, mut harness) = window(&archive_with(large().build()), Language::BritishEnglish)?;

    for path in ["", "libs", "libs/items", "libs/items/group599"] {
        let browser = harness.state_mut().browser_mut().ok_or("no Archive")?;
        let node = browser
            .records()
            .and_then(|tree| tree.find(path))
            .ok_or_else(|| format!("no node {path}"))?;
        browser.toggle_record_node(node, || {});
        wait_until(&mut harness, settled)?;
    }

    let tree = harness
        .state()
        .browser()
        .and_then(Browser::records)
        .ok_or("no node")?;
    assert_eq!(
        tree.rows().len(),
        3 + 600 + 200,
        "only open folders are rows"
    );
    let last = tree.rows().last().map(|row| tree.node(row.node).name());
    assert_eq!(last, Some("item119999"), "sorted by name");
    assert!(
        harness.query_by_label("item119999").is_none(),
        "rows outside the view are not drawn"
    );
    Ok(())
}

fn held_load(harness: &mut Harness<'_, Model>) -> Result<p4kx_gui::Hold, Box<dyn Error>> {
    harness
        .state_mut()
        .browser_mut()
        .and_then(Browser::hold_record_load)
        .ok_or_else(|| "no DataForge to hold".into())
}

#[test]
fn a_running_load_is_dropped_on_reload_and_on_a_change() -> TestResult {
    let (_dir, path, mut harness) =
        window(&archive_with(large().build()), Language::BritishEnglish)?;
    let status_after = |harness: &mut Harness<'_, Model>| {
        std::thread::sleep(Duration::from_millis(500));
        harness.run_quiet();
        harness
            .state()
            .browser()
            .and_then(Browser::records)
            .map(|tree| (format!("{:?}", tree.status()), tree.rows().len()))
    };

    let first_hold = held_load(&mut harness)?;
    harness.get_by_label("Expand DataForge").click();
    harness.run_steps(2);
    let first_loading = loading(harness.state());
    harness.state_mut().reload(|| {});
    wait_until(&mut harness, |model| model.opening_path().is_none())?;
    drop(first_hold);
    let after_reload = status_after(&mut harness);
    let second_hold = held_load(&mut harness)?;
    harness.get_by_label("Expand DataForge").click();
    harness.run_steps(2);
    let second_loading = loading(harness.state());
    archive_with(database().build()).write_to(&path)?;
    harness.state_mut().check_archive();
    drop(second_hold);
    let after_change = status_after(&mut harness);

    assert!(first_loading && second_loading, "both loads were running");
    assert_eq!(after_reload, Some(("NotLoaded".to_owned(), 1)), "reloaded");
    assert_eq!(
        after_change,
        Some(("ArchiveChanged".to_owned(), 1)),
        "changed"
    );
    Ok(())
}
