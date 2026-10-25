// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Searching the Records of a synthetic DataForge by record path and type name.

use crate::waiting::wait_polling;
use std::error::Error;
use std::time::{Duration, Instant};

use p4kx_gui::{Browser, Language, Model, RecordTree};
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
            "libs/foundry/records/ammo/laser.xml",
        ))
        .record(TestRecord::new(
            0x13,
            "AmmoParams",
            "AmmoParams.Ballistic_S1",
            "libs/foundry/records/ammo/ballistic.xml",
        ))
        .record(TestRecord::new(
            0x14,
            "EntityClassDefinition",
            "EntityClassDefinition.ANVL_Arrow",
            "libs/foundry/records/entities/anvl_arrow.xml",
        ))
}

fn opened(bytes: Vec<u8>) -> Result<(TempDir, Model), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Libs\\config.xml", "config"))
        .entry(TestEntry::zstd("Data\\Game2.dcb", bytes))
        .write_to(&path)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path, || {});
    settle(&mut model, |model| model.browser().is_some())?;
    Ok((dir, model))
}

fn settle(model: &mut Model, done: impl Fn(&Model) -> bool) -> TestResult {
    wait_polling(
        model,
        "the work to finish",
        |model| {
            model.poll();
            if let Some(browser) = model.browser_mut() {
                browser.poll();
            }
        },
        done,
    )
}

fn search(model: &mut Model, text: &str) -> Result<Vec<String>, Box<dyn Error>> {
    model
        .browser_mut()
        .ok_or("no Archive")?
        .search_records(text, || {});
    settle(model, |model| {
        model
            .browser()
            .is_some_and(|browser| !browser.is_searching_records())
    })?;
    let browser = model.browser().ok_or("no Archive")?;
    let tree = browser.records().ok_or("no DataForge")?;
    let matches = browser.record_matches().ok_or("no matches")?;
    Ok(matches
        .iter()
        .filter_map(|&id| tree.record(id))
        .map(|record| record.name().to_owned())
        .collect())
}

#[test]
fn a_part_of_the_record_path_finds_records_in_record_path_order() -> TestResult {
    let (_dir, mut model) = opened(database().build())?;

    let found = search(&mut model, "AMMO/")?;

    assert_eq!(
        found,
        ["AmmoParams.Ballistic_S1", "AmmoParams.Laser_S1"],
        "both ammo Records, ignoring case, sorted by record path"
    );
    Ok(())
}

#[test]
fn the_type_name_finds_records_too() -> TestResult {
    let (_dir, mut model) = opened(database().build())?;

    let found = search(&mut model, "entityclass")?;

    assert_eq!(
        found,
        [
            "EntityClassDefinition.AEGS_Avenger",
            "EntityClassDefinition.ANVL_Arrow"
        ],
        "the Records of that type"
    );
    Ok(())
}

#[test]
fn a_glob_matches_the_whole_record_path_or_type_with_either_separator() -> TestResult {
    let (_dir, mut model) = opened(database().build())?;

    let by_path = search(&mut model, "libs\\*\\a*_a*")?;
    let by_type = search(&mut model, "Ammo*")?;
    let partial = search(&mut model, "Ammo?")?;

    assert_eq!(
        by_path,
        [
            "EntityClassDefinition.AEGS_Avenger",
            "EntityClassDefinition.ANVL_Arrow"
        ],
        "a glob over the record path"
    );
    assert_eq!(
        by_type,
        ["AmmoParams.Ballistic_S1", "AmmoParams.Laser_S1"],
        "a glob over the type name"
    );
    assert!(partial.is_empty(), "a glob must match the whole text");
    Ok(())
}

#[test]
fn the_record_name_alone_does_not_match() -> TestResult {
    let (_dir, mut model) = opened(database().build())?;

    let found = search(&mut model, "Laser_S1")?;

    assert!(found.is_empty(), "only record path and type are searched");
    Ok(())
}

#[test]
fn searching_loads_dataforge_first_and_empty_text_ends_the_search() -> TestResult {
    let (_dir, mut model) = opened(database().build())?;
    let before = model
        .browser()
        .and_then(Browser::records)
        .map(|tree| format!("{:?}", tree.status()));

    let found = search(&mut model, "arrow")?;
    let browser = model.browser_mut().ok_or("no Archive")?;
    browser.search_records("", || {});

    assert_eq!(
        before.as_deref(),
        Some("NotLoaded"),
        "nothing loaded before"
    );
    assert_eq!(
        found,
        ["EntityClassDefinition.ANVL_Arrow"],
        "found once loaded"
    );
    assert!(
        browser.record_matches().is_none(),
        "no matches without text"
    );
    assert!(!browser.is_searching_records(), "nothing runs");
    Ok(())
}

#[test]
fn picking_a_match_selects_it_in_the_expanded_tree() -> TestResult {
    let (_dir, mut model) = opened(database().build())?;
    search(&mut model, "anvl")?;
    let browser = model.browser_mut().ok_or("no Archive")?;
    let picked = *browser
        .record_matches()
        .and_then(<[_]>::first)
        .ok_or("no match")?;

    browser.pick_record(picked);

    let tree = browser.records().ok_or("no DataForge")?;
    let node = tree.record_node(picked).ok_or("no node")?;
    assert_eq!(tree.selected(), Some(node), "the Record is selected");
    let shown: Vec<&str> = tree
        .rows()
        .iter()
        .map(|row| tree.node(row.node).name())
        .collect();
    assert_eq!(
        shown,
        [
            "",
            "libs",
            "foundry",
            "records",
            "ammo",
            "entities",
            "aegs_avenger",
            "anvl_arrow"
        ],
        "the folders above it are expanded"
    );
    assert!(
        browser.shows_records(),
        "the list panel shows the Record search"
    );
    Ok(())
}

#[test]
fn an_unreadable_dataforge_ends_the_search_without_matches() -> TestResult {
    let (_dir, mut model) = opened(database().version(9).build())?;

    let browser = model.browser_mut().ok_or("no Archive")?;
    browser.search_records("ammo", || {});
    settle(&mut model, |model| {
        model
            .browser()
            .is_some_and(|browser| !browser.is_searching_records())
    })?;

    let browser = model.browser().ok_or("no Archive")?;
    assert!(browser.record_matches().is_none(), "no matches");
    assert!(
        browser
            .records()
            .is_some_and(|tree: &RecordTree| tree.selected().is_none()),
        "nothing selected"
    );
    Ok(())
}

#[test]
fn a_newer_search_replaces_a_running_one_over_many_records() -> TestResult {
    let large = (0..120_000_u128).fold(
        DataForgeBuilder::new().structure(TestStruct::new("Item")),
        |builder, n| {
            builder.record(TestRecord::new(
                n + 1,
                "Item",
                format!("Item.N{n}"),
                format!("libs/items/group{:03}/item{n:06}.xml", n % 600),
            ))
        },
    );
    let (_dir, mut model) = opened(large.build())?;
    search(&mut model, "group001")?;

    let browser = model.browser_mut().ok_or("no Archive")?;
    browser.search_records("group", || {});
    let found = search(&mut model, "item119999")?;

    assert_eq!(found, ["Item.N119999"], "only the last search counts");
    Ok(())
}

#[test]
fn the_search_runs_off_the_calling_thread() -> TestResult {
    let large = (0..120_000_u128).fold(
        DataForgeBuilder::new().structure(TestStruct::new("Item")),
        |builder, n| {
            builder.record(TestRecord::new(
                n + 1,
                "Item",
                format!("Item.N{n}"),
                format!("libs/items/item{n:06}.xml"),
            ))
        },
    );
    let (_dir, mut model) = opened(large.build())?;
    search(&mut model, "item000001")?;

    let browser = model.browser_mut().ok_or("no Archive")?;
    let started = Instant::now();
    browser.search_records("*item*", || {});
    let returned = started.elapsed();
    let running = browser.is_searching_records();
    let count = search(&mut model, "*item*")?.len();

    assert!(
        running,
        "the search was still running when the call returned"
    );
    assert!(
        returned < Duration::from_millis(50),
        "returned in {returned:?}"
    );
    assert_eq!(count, 120_000, "every Record matched");
    Ok(())
}

#[test]
fn records_are_searched_again_after_a_reload() -> TestResult {
    let (_dir, mut model) = opened(database().build())?;
    search(&mut model, "ammo")?;

    model.reload(|| {});
    settle(&mut model, |model| {
        model.opening_path().is_none() && model.browser().is_some()
    })?;
    let browser = model.browser().ok_or("no Archive")?;
    let cleared = browser.record_matches().is_none() && browser.record_search_text().is_empty();
    let found = search(&mut model, "arrow")?;

    assert!(cleared, "a reload starts afresh");
    assert_eq!(
        found,
        ["EntityClassDefinition.ANVL_Arrow"],
        "searchable again"
    );
    Ok(())
}

#[test]
fn a_change_on_disk_ends_the_search() -> TestResult {
    let (dir, mut model) = opened(database().build())?;
    search(&mut model, "ammo")?;

    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Libs\\config.xml", "changed"))
        .entry(TestEntry::zstd("Data\\Game2.dcb", database().build()))
        .write_to(dir.path().join("Data.p4k"))?;
    model.check_archive();

    let browser = model.browser().ok_or("no Archive")?;
    assert!(browser.record_matches().is_none(), "no matches");
    assert!(browser.record_search_text().is_empty(), "no text");
    assert!(!browser.is_searching_records(), "nothing runs");
    Ok(())
}
