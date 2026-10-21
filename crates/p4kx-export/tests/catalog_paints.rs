//! Paints and the ships they fit in the Basetool catalog, through the public API.

mod paints_fixture;

use std::collections::BTreeSet;
use std::error::Error;
use std::time::{Duration, UNIX_EPOCH};

use p4kx_export::{BuiltCatalog, CatalogOptions, Filter, Kind, PaintSummary, build_catalog};
use p4kx_extraction::Archive;
use paints_fixture::{
    PAINT_ALPHA_RED, PAINT_ALPHA_VARIANT, PAINT_BETA, PAINT_GAMMA, PAINT_ORPHAN, PAINT_UPPER,
    SHIP_ALPHA, SHIP_BETA, SHIP_GAMMA, write_archive,
};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const EXPORTED_AT: Duration = Duration::from_secs(1_792_000_000);

fn build(filter: Option<&Filter>) -> Result<(BuiltCatalog, Value), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive =
        Archive::open_with_key(write_archive(dir.path())?, Some(p4kx_testkit::test_key()))?;
    let mut options = CatalogOptions::new(UNIX_EPOCH + EXPORTED_AT);
    options.filter = filter;
    let built = build_catalog(&archive, &options)?;
    let json = serde_json::from_slice(built.json())?;
    Ok((built, json))
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

fn ships_of(catalog: &Value, paint: u128) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let wanted = guid(paint);
    let entry = catalog["paints"]
        .as_array()
        .ok_or("paints")?
        .iter()
        .find(|entry| entry["guid"] == wanted.as_str())
        .ok_or("the paint")?;
    Ok(entry["shipGuids"]
        .as_array()
        .ok_or("shipGuids")?
        .iter()
        .filter_map(|ship| ship.as_str().map(str::to_owned))
        .collect())
}

fn set(ships: &[u128]) -> BTreeSet<String> {
    ships.iter().copied().map(guid).collect()
}

#[test]
fn a_paint_fits_the_ships_whose_paint_port_requires_its_tags() -> TestResult {
    let (_, catalog) = build(None)?;
    assert_eq!(
        ships_of(&catalog, PAINT_ALPHA_RED)?,
        set(&[SHIP_ALPHA]),
        "a required tag among the paint's tags; the Modification Beta moves the other ship"
    );
    assert_eq!(
        ships_of(&catalog, PAINT_BETA)?,
        set(&[SHIP_BETA]),
        "the Elem of the Modification overrides the required tags of the port by id"
    );
    assert_eq!(
        ships_of(&catalog, PAINT_GAMMA)?,
        set(&[SHIP_GAMMA]),
        "a paint port in the Record of the ship"
    );
    Ok(())
}

#[test]
fn a_tag_below_the_required_tag_in_the_tag_tree_and_a_tag_in_other_case_match() -> TestResult {
    let (_, catalog) = build(None)?;
    assert_eq!(
        ships_of(&catalog, PAINT_ALPHA_VARIANT)?,
        set(&[SHIP_ALPHA]),
        "Paint_Alpha_Variant lies below Paint_Alpha"
    );
    assert_eq!(
        ships_of(&catalog, PAINT_UPPER)?,
        set(&[SHIP_ALPHA]),
        "tag names compare ignoring case"
    );
    Ok(())
}

#[test]
fn a_port_for_other_types_assigns_no_paint() -> TestResult {
    let (_, catalog) = build(None)?;
    assert!(
        ships_of(&catalog, PAINT_ORPHAN)?.is_empty(),
        "the port that requires Paint_Orphan takes weapons"
    );
    Ok(())
}

#[test]
fn the_summary_counts_the_paints_with_and_without_a_ship() -> TestResult {
    let (built, catalog) = build(None)?;
    assert_eq!(
        built.summary().paints(),
        PaintSummary {
            paints: 6,
            assigned: 5,
            unassigned: 1,
            ships: 3,
            unreadable_vehicles: 1,
        },
        "the corrupt vehicle file of one ship is counted"
    );
    let kind = built.summary().kind(Kind::Paints).ok_or("the kind")?;
    assert_eq!((kind.count, kind.complete), (6, true), "the kind summary");
    assert_eq!(
        catalog["completeness"]["paints"]["status"], "complete",
        "completeness"
    );
    Ok(())
}

#[test]
fn every_ship_of_an_assignment_is_a_catalog_ship() -> TestResult {
    let (_, catalog) = build(None)?;
    let ships: BTreeSet<&str> = catalog["ships"]
        .as_array()
        .ok_or("ships")?
        .iter()
        .filter_map(|ship| ship["guid"].as_str())
        .collect();
    let items: BTreeSet<&str> = catalog["items"]
        .as_array()
        .ok_or("items")?
        .iter()
        .filter_map(|item| item["guid"].as_str())
        .collect();
    for paint in catalog["paints"].as_array().ok_or("paints")? {
        assert!(
            items.contains(paint["guid"].as_str().ok_or("guid")?),
            "an item"
        );
        let listed: Vec<&str> = paint["shipGuids"]
            .as_array()
            .ok_or("shipGuids")?
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert!(listed.is_sorted(), "ordered by GUID");
        assert!(listed.iter().all(|ship| ships.contains(ship)), "resolves");
    }
    Ok(())
}

#[test]
fn paints_are_ordered_by_guid_and_carry_a_name_but_no_description() -> TestResult {
    let (_, catalog) = build(None)?;
    let paints = catalog["paints"].as_array().ok_or("paints")?;
    let guids: Vec<&str> = paints
        .iter()
        .filter_map(|paint| paint["guid"].as_str())
        .collect();
    assert!(guids.is_sorted(), "ordered by GUID");
    let red = &paints[0];
    assert_eq!(red["className"], "paint_alpha_red", "class name");
    assert_eq!(red["name"]["en"], "Alpha Red", "short name");
    let mut fields: Vec<&str> = red
        .as_object()
        .ok_or("object")?
        .keys()
        .map(String::as_str)
        .collect();
    fields.sort_unstable();
    assert_eq!(
        fields,
        ["className", "guid", "name", "recordPath", "shipGuids"],
        "no description, no media"
    );
    Ok(())
}

#[test]
fn a_development_leftover_is_neither_an_item_nor_a_paint() -> TestResult {
    let (_, catalog) = build(None)?;
    for kind in ["items", "paints"] {
        assert!(
            catalog[kind]
                .as_array()
                .ok_or("kind")?
                .iter()
                .all(|entry| entry["className"] != "paint_test"),
            "{kind} leaves out paint_test"
        );
    }
    Ok(())
}

#[test]
fn a_filter_makes_the_paints_partial_and_assigns_only_the_selected_ships() -> TestResult {
    let filter = Filter::new("paint_beta");
    let (built, catalog) = build(Some(&filter))?;
    assert_eq!(
        catalog["completeness"]["paints"]["status"], "partial",
        "partial"
    );
    assert_eq!(
        catalog["paints"].as_array().ok_or("paints")?.len(),
        1,
        "only the paint the Filter matches"
    );
    assert!(
        ships_of(&catalog, PAINT_BETA)?.is_empty(),
        "the ships are filtered as well, so none of them is listed"
    );
    assert_eq!(built.summary().paints().unassigned, 1, "counted");
    Ok(())
}
