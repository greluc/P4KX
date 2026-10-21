//! Paints and their ship assignment in the Basetool catalog of a real Archive; runs with
//! `--ignored` when `P4KX_DATA_P4K` names one.

use std::collections::{BTreeSet, HashSet};
use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{BuiltCatalog, CatalogOptions, Kind, build_catalog};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const MIN_PAINTS: usize = 900;
const MIN_ASSIGNED: usize = 900;
const PAINT_100I_BLACK_ORANGE: &str = "bc3e5699-023e-4eda-95d9-202e48d7dd94";
const ORIG_100I: &str = "6135a874-4cb1-4f49-9f29-5781e5991f2b";
const ORIG_125A: &str = "e7eeba8d-b108-48a1-a332-d9edbab2b9c1";
const ORIG_135C: &str = "2f231daa-95e5-4cb2-a32e-4db8f29a30d6";
const PAINT_MUSTANG_OMEGA: &str = "de95879d-ebaf-49c6-8016-ebfbd1d8d7bb";
const CNOU_MUSTANG_OMEGA: &str = "1164be4d-623e-4ef7-bc3d-4cdb45c22ec5";

fn list<'c>(catalog: &'c Value, kind: &str) -> Result<&'c [Value], Box<dyn Error>> {
    Ok(catalog[kind].as_array().ok_or("the kind is a list")?)
}

fn guids<'c>(entry: &'c Value, field: &str) -> Result<Vec<&'c str>, Box<dyn Error>> {
    Ok(entry[field]
        .as_array()
        .ok_or("a list")?
        .iter()
        .filter_map(Value::as_str)
        .collect())
}

fn ships_of<'c>(paints: &'c [Value], guid: &str) -> Result<BTreeSet<&'c str>, Box<dyn Error>> {
    let paint = paints
        .iter()
        .find(|paint| paint["itemGuid"] == guid)
        .ok_or("the paint")?;
    Ok(guids(paint, "shipGuids")?.into_iter().collect())
}

#[expect(
    clippy::print_stdout,
    reason = "the counts are the result of the verification"
)]
fn report(built: &BuiltCatalog) {
    let counts = built.summary().paints();
    println!(
        "paints: {} assigned {} unassigned {} ships with paints {} unreadable vehicles {}",
        counts.paints, counts.assigned, counts.unassigned, counts.ships, counts.unreadable_vehicles
    );
}

fn check_counts(built: &BuiltCatalog, catalog: &Value) -> TestResult {
    let counts = built.summary().paints();
    let paints = list(catalog, "paints")?;
    assert!(paints.len() >= MIN_PAINTS, "{} paints", paints.len());
    assert_eq!(paints.len(), counts.paints, "the summary counts the paints");
    assert!(
        counts.assigned >= MIN_ASSIGNED,
        "{} assigned",
        counts.assigned
    );
    assert_eq!(
        counts.assigned + counts.unassigned,
        counts.paints,
        "assigned and unassigned add up"
    );
    let without_ship = paints
        .iter()
        .filter(|paint| paint["shipGuids"].as_array().is_some_and(Vec::is_empty))
        .count();
    assert_eq!(without_ship, counts.unassigned, "the paints without a ship");
    assert_eq!(counts.unreadable_vehicles, 0, "every vehicle file reads");
    assert_eq!(
        built.summary().kind(Kind::Paints).map(|kind| kind.count),
        Some(paints.len()),
        "the kind summary"
    );
    Ok(())
}

fn check_references(built: &BuiltCatalog, catalog: &Value) -> TestResult {
    let ships: HashSet<&str> = list(catalog, "ships")?
        .iter()
        .filter_map(|ship| ship["guid"].as_str())
        .collect();
    let items: HashSet<&str> = list(catalog, "items")?
        .iter()
        .filter_map(|item| item["guid"].as_str())
        .collect();
    let mut used = HashSet::new();
    for paint in list(catalog, "paints")? {
        let item = paint["itemGuid"].as_str().ok_or("itemGuid")?;
        assert!(items.contains(item), "paint {item} is an item");
        let listed = guids(paint, "shipGuids")?;
        assert!(listed.is_sorted(), "ship GUIDs of {item} are ordered");
        for ship in listed {
            assert!(ships.contains(ship), "ship {ship} of paint {item} resolves");
            used.insert(ship);
        }
    }
    assert_eq!(
        used.len(),
        built.summary().paints().ships,
        "the summary counts the ships"
    );
    Ok(())
}

fn check_known_paints(catalog: &Value) -> TestResult {
    let paints = list(catalog, "paints")?;
    let series = ships_of(paints, PAINT_100I_BLACK_ORANGE)?;
    for ship in [ORIG_100I, ORIG_125A, ORIG_135C] {
        assert!(series.contains(ship), "the 100i paint fits {ship}");
    }
    assert_eq!(
        ships_of(paints, PAINT_MUSTANG_OMEGA)?,
        BTreeSet::from([CNOU_MUSTANG_OMEGA]),
        "the Omega paint fits the Omega alone: its Modification sets the required tag"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_paints_of_a_real_archive_fit_their_ships() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    report(&built);
    check_counts(&built, &catalog)?;
    check_references(&built, &catalog)?;
    check_known_paints(&catalog)
}
