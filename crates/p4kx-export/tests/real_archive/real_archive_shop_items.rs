//! Item lists of shops in the catalog of a real Archive; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one. The diagnostics print before any assertion.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, build_catalog};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const KINDS_WITH_ITEMS: [&str; 5] = ["items", "ships", "commodities", "paints", "loadouts"];

fn guids_of(catalog: &Value) -> HashSet<&str> {
    KINDS_WITH_ITEMS
        .iter()
        .filter_map(|kind| catalog[*kind].as_array())
        .flatten()
        .filter_map(|entry| entry["guid"].as_str())
        .collect()
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    clippy::too_many_lines,
    reason = "the diagnostics are the result of the verification, printed before the assertions"
)]
fn shops_get_item_lists_from_the_inventories_their_containers_name() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    let shops = catalog["shops"].as_array().ok_or("shops")?;
    let summary = built.summary().shops();
    println!("== summary {summary:#?}");
    let with: Vec<&Value> = shops
        .iter()
        .filter(|shop| !shop["itemGuids"].is_null())
        .collect();
    let mut by_inventory: BTreeMap<&str, usize> = BTreeMap::new();
    for shop in &with {
        *by_inventory
            .entry(shop["inventory"].as_str().unwrap_or("(none)"))
            .or_default() += 1;
    }
    println!(
        "== shops with an item list {} of {}; distinct inventories {}",
        with.len(),
        shops.len(),
        by_inventory.len()
    );
    for (name, count) in &by_inventory {
        println!("   {name}: {count} shops");
    }
    let known = guids_of(&catalog);
    let mut total = 0_usize;
    let mut resolved = 0_usize;
    let mut unresolved: BTreeSet<&str> = BTreeSet::new();
    let mut empty = 0_usize;
    let mut inventories_seen: HashSet<&str> = HashSet::new();
    for shop in &with {
        let items = shop["itemGuids"].as_array().ok_or("itemGuids")?;
        empty += usize::from(items.is_empty());
        if !inventories_seen.insert(shop["inventory"].as_str().unwrap_or_default()) {
            continue;
        }
        for item in items.iter().filter_map(Value::as_str) {
            total += 1;
            if known.contains(item) {
                resolved += 1;
            } else {
                unresolved.insert(item);
            }
        }
    }
    println!(
        "== item GUIDs over distinct inventories: {total}, found in items/ships/commodities/paints/loadouts: {resolved}, distinct not found: {}, empty lists: {empty}",
        unresolved.len()
    );
    for item in unresolved.iter().take(40) {
        println!("   not found {item}");
    }
    let by_system: BTreeMap<&str, usize> = with.iter().fold(BTreeMap::new(), |mut counts, shop| {
        *counts
            .entry(shop["system"].as_str().unwrap_or("(none)"))
            .or_default() += 1;
        counts
    });
    println!("== shops with an item list per system {by_system:?}");
    let text = serde_json::to_string(&catalog["shops"])?;
    for field in [
        "BuyPrice",
        "SellPrice",
        "CurrentInventory",
        "RentalOfferings",
    ] {
        assert!(!text.contains(field), "{field} leaked into the shops");
    }
    assert!(summary.with_items > 0, "{summary:?}");
    assert_eq!(summary.with_items, with.len());
    assert_eq!(summary.inventory_unreadable, 0, "{summary:?}");
    assert!(resolved * 10 >= total * 9, "{resolved} of {total} resolve");
    Ok(())
}
