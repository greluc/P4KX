// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Record search in the list panel, driven headless over a synthetic DataForge.

use std::error::Error;
use std::time::{Duration, Instant};

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Browser, Language, Model, PreviewTab, RecordTree, RecordTreeStatus, ui};
use p4kx_testkit::{ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

fn database() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(TestStruct::new("EntityClassDefinition"))
        .structure(TestStruct::new("AmmoParams"))
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

fn window(
    database: &DataForgeBuilder,
    language: Language,
) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Libs\\config.xml", "config"))
        .entry(TestEntry::zstd("Data\\Game2.dcb", database.build()))
        .write_to(&path)?;
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
    wait_until(&mut harness, |model| model.browser().is_some())?;
    Ok((dir, harness))
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
    harness.run_steps(3);
    Ok(())
}

fn searched(model: &Model) -> bool {
    model.browser().is_some_and(|browser| {
        !browser.is_searching_records() && browser.record_matches().is_some()
    })
}

fn type_search(harness: &mut Harness<'_, Model>, text: &str) {
    harness.get_by_role(Role::TextInput).click();
    harness.run_steps(2);
    for character in text.chars() {
        harness
            .get_by_role(Role::TextInput)
            .type_text(&character.to_string());
        harness.run_steps(2);
    }
}

fn open_records(harness: &mut Harness<'_, Model>) {
    harness
        .get_by_role_and_label(Role::Button, "DataForge")
        .click();
    harness.run_steps(2);
}

#[test]
fn selecting_the_dataforge_node_offers_the_record_search() -> TestResult {
    let (_dir, mut harness) = window(&database(), Language::BritishEnglish)?;

    open_records(&mut harness);

    assert!(harness.query_by_label("Search Records").is_some(), "label");
    assert!(
        harness
            .query_by_label("Type part of a record path or type name to find Records.")
            .is_some(),
        "prompt"
    );
    assert!(
        harness.query_by_label("Quick filters").is_none(),
        "no Entry filters"
    );
    Ok(())
}

#[test]
fn typing_lists_matching_records_with_record_path_and_type() -> TestResult {
    let (_dir, mut harness) = window(&database(), Language::BritishEnglish)?;
    open_records(&mut harness);

    type_search(&mut harness, "ammo");
    wait_until(&mut harness, searched)?;

    harness.get_by_role_and_label(Role::Tab, "Text").click();
    harness.run_steps(2);

    for head in ["Name", "Type", "Record path"] {
        assert!(harness.query_by_label(head).is_some(), "{head}");
    }
    assert!(harness.query_by_label("2 Records").is_some(), "the count");
    for name in ["AmmoParams.Ballistic_S1", "AmmoParams.Laser_S1"] {
        assert!(
            harness
                .query_by_role_and_label(Role::Button, name)
                .is_some(),
            "the row of {name}"
        );
    }
    assert_eq!(
        harness
            .query_all_by_label("libs/foundry/records/ammo")
            .count(),
        2,
        "the record path of each"
    );
    assert_eq!(
        harness.query_all_by_label("AmmoParams").count(),
        2,
        "the type of each"
    );
    assert!(
        harness
            .query_by_label("EntityClassDefinition.AEGS_Avenger")
            .is_none(),
        "no other Record"
    );
    Ok(())
}

#[test]
fn picking_a_match_selects_the_record_in_the_tree_and_the_preview() -> TestResult {
    let (_dir, mut harness) = window(&database(), Language::BritishEnglish)?;
    open_records(&mut harness);
    type_search(&mut harness, "entityclass");
    wait_until(&mut harness, searched)?;

    harness
        .get_by_role_and_label(Role::Button, "EntityClassDefinition.AEGS_Avenger")
        .click();
    harness.run_steps(3);

    let browser = harness.state().browser().ok_or("no Archive")?;
    let tree = browser.records().ok_or("no DataForge")?;
    let selected = tree.selected().map(|node| tree.node(node).name());
    assert_eq!(selected, Some("aegs_avenger"), "the Record in the tree");
    assert_eq!(
        harness.state().preview_tab(),
        PreviewTab::Record,
        "the Record preview"
    );
    assert!(
        harness
            .query_all_by_role_and_label(Role::Button, "aegs_avenger")
            .count()
            == 1,
        "the tree row is shown"
    );
    assert!(harness.query_by_label("1 Record").is_some(), "matches stay");
    Ok(())
}

#[test]
fn a_match_can_be_picked_with_the_keyboard() -> TestResult {
    let (_dir, mut harness) = window(&database(), Language::BritishEnglish)?;
    open_records(&mut harness);
    type_search(&mut harness, "ammo");
    wait_until(&mut harness, searched)?;

    harness
        .get_by_role_and_label(Role::Button, "AmmoParams.Laser_S1")
        .focus();
    harness.run_steps(2);
    harness.key_press(egui::Key::Enter);
    harness.run_steps(3);

    let tree = harness
        .state()
        .browser()
        .and_then(Browser::records)
        .ok_or("no DataForge")?;
    let selected = tree.selected().map(|node| tree.node(node).name());
    assert_eq!(selected, Some("AmmoParams.Laser_S1"), "picked with Enter");
    Ok(())
}

#[test]
fn nothing_found_says_so() -> TestResult {
    let (_dir, mut harness) = window(&database(), Language::BritishEnglish)?;
    open_records(&mut harness);

    type_search(&mut harness, "nothing-like-this");
    wait_until(&mut harness, searched)?;

    assert!(
        harness
            .query_by_label("No Record matches the search.")
            .is_some(),
        "the note"
    );
    assert!(harness.query_by_label("0 Records").is_some(), "the count");
    Ok(())
}

#[test]
fn selecting_a_folder_returns_to_the_entries() -> TestResult {
    let (_dir, mut harness) = window(&database(), Language::BritishEnglish)?;
    open_records(&mut harness);
    type_search(&mut harness, "ammo");
    wait_until(&mut harness, searched)?;

    harness.get_by_role_and_label(Role::Button, "Data").click();
    harness.run_steps(3);

    assert!(
        harness.query_by_label("Search Records").is_none(),
        "Records gone"
    );
    assert!(
        harness.query_by_label("config.xml").is_some(),
        "the Entries"
    );
    Ok(())
}

#[test]
fn the_german_search_names_its_parts() -> TestResult {
    let (_dir, mut harness) = window(&database(), Language::German)?;
    open_records(&mut harness);

    type_search(&mut harness, "ammo");
    wait_until(&mut harness, searched)?;

    for label in ["Records suchen", "2 Records"] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    for head in ["Typ", "Record-Pfad"] {
        assert_eq!(harness.query_all_by_label(head).count(), 2, "{head}");
    }
    Ok(())
}

#[test]
fn the_window_answers_while_many_records_are_searched() -> TestResult {
    let (_dir, mut harness) = window(&large(), Language::BritishEnglish)?;
    open_records(&mut harness);
    wait_until(&mut harness, |model| {
        model
            .browser()
            .and_then(Browser::records)
            .is_some_and(|tree: &RecordTree| matches!(tree.status(), RecordTreeStatus::Ready(_)))
    })?;

    type_search(&mut harness, "*item*");
    let started = Instant::now();
    harness.step();
    let frame = started.elapsed();
    wait_until(&mut harness, searched)?;

    assert!(frame < Duration::from_millis(250), "a frame took {frame:?}");
    assert!(
        harness.query_by_label("120,000 Records").is_some(),
        "every Record"
    );
    assert!(
        harness.query_by_label("Item.N119999").is_none(),
        "rows outside the view are not drawn"
    );
    Ok(())
}

#[test]
fn picking_a_match_far_down_scrolls_the_tree_to_it() -> TestResult {
    let (_dir, mut harness) = window(&large(), Language::BritishEnglish)?;
    open_records(&mut harness);
    type_search(&mut harness, "group599/item119999");
    wait_until(&mut harness, searched)?;
    let before = harness
        .query_all_by_role_and_label(Role::Button, "item119999")
        .count();

    harness
        .get_by_role_and_label(Role::Button, "Item.N119999")
        .click();
    harness.run_steps(4);

    assert_eq!(before, 0, "not in the tree before");
    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Button, "item119999")
            .count(),
        1,
        "the tree row is drawn, so the tree scrolled to it"
    );
    Ok(())
}
