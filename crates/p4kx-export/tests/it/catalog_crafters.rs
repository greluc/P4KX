// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The crafting stations in the Basetool catalog, through the public API.

use crate::crafters_fixture::{BENCH, FABRICATOR, write_archive};

use std::error::Error;
use std::time::{Duration, UNIX_EPOCH};

use p4kx_export::{BuiltCatalog, CatalogOptions, Filter, Kind, build_catalog};
use p4kx_extraction::Archive;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn build(filter: Option<&Filter>) -> Result<(BuiltCatalog, Value), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive =
        Archive::open_with_key(write_archive(dir.path())?, Some(p4kx_testkit::test_key()))?;
    let mut options = CatalogOptions::new(UNIX_EPOCH + Duration::from_secs(1_792_000_000));
    options.filter = filter;
    let built = build_catalog(&archive, &options)?;
    let catalog = serde_json::from_slice(built.json())?;
    Ok((built, catalog))
}

fn guid(value: u128) -> String {
    let text = format!("{value:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &text[0..8],
        &text[8..12],
        &text[12..16],
        &text[16..20],
        &text[20..32]
    )
}

fn class_names<'a>(catalog: &'a Value, kind: &str) -> Vec<&'a str> {
    catalog[kind]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry["className"].as_str())
        .collect()
}

#[test]
fn every_entity_with_a_crafter_component_is_a_crafter_whatever_its_kind() -> TestResult {
    let (built, catalog) = build(None)?;
    assert_eq!(
        class_names(&catalog, "crafters"),
        ["bench", "fabricator", "crafting_item", "crafting_commodity"],
        "ordered by GUID; the entity without the component and the template are absent"
    );
    assert!(
        class_names(&catalog, "items").contains(&"crafting_item"),
        "a crafting item stays an item"
    );
    assert!(
        class_names(&catalog, "commodities").contains(&"crafting_commodity"),
        "a crafting commodity stays a commodity"
    );
    let kind = built.summary().kind(Kind::Crafters);
    assert_eq!(
        kind.map(|kind| (kind.count, kind.skipped, kind.complete)),
        Some((4, 1, true)),
        "the template is the one skipped; the plain template holds no component"
    );
    Ok(())
}

#[test]
fn a_crafter_carries_the_limits_of_its_component_as_stored() -> TestResult {
    let (_, catalog) = build(None)?;
    let entry = catalog["crafters"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|entry| entry["guid"] == guid(BENCH).as_str())
        .ok_or("the crafter")?;
    assert_eq!(
        *entry,
        json!({
            "guid": guid(BENCH),
            "className": "bench",
            "recordPath": "libs/foundry/records/entities/basebuilding/interactables/bench",
            "maxJobsInHistory": 10,
            "queues": [
                {"name": "Fabricator", "maxJobsInProgress": 2, "maxJobsWaiting": 4},
                {"name": "Workbench", "maxJobsInProgress": 4, "maxJobsWaiting": 3},
            ],
        })
    );
    Ok(())
}

#[test]
fn a_crafter_without_queues_has_an_empty_list() -> TestResult {
    let (_, catalog) = build(None)?;
    let entry = catalog["crafters"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|entry| entry["guid"] == guid(FABRICATOR).as_str())
        .ok_or("the crafter")?;
    assert_eq!(entry["queues"], json!([]));
    assert_eq!(entry["maxJobsInHistory"], 0);
    Ok(())
}

#[test]
fn a_filter_keeps_the_crafters_whose_path_it_matches() -> TestResult {
    let filter = Filter::new("*/basebuilding/*");
    let (built, catalog) = build(Some(&filter))?;
    assert_eq!(
        class_names(&catalog, "crafters"),
        ["bench", "fabricator"],
        "the item and the commodity are outside the filter"
    );
    assert_eq!(
        catalog["completeness"]["crafters"],
        json!({"status": "partial", "filter": "*/basebuilding/*"})
    );
    assert!(
        built
            .summary()
            .kind(Kind::Crafters)
            .is_some_and(|kind| !kind.complete),
        "the summary agrees"
    );
    Ok(())
}
