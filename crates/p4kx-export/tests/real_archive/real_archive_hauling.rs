// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Hauling orders of contracts and cargo manifests in the Basetool catalog of a real Archive; runs
//! with `--ignored` when `P4KX_DATA_P4K` names one.

use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, build_catalog};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

#[expect(
    clippy::print_stdout,
    reason = "the figures are the result of the verification"
)]
fn print_line(line: &str) {
    println!("{line}");
}

fn items(order: &Value) -> impl Iterator<Item = &Value> {
    order["requirements"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|requirement| requirement["options"].as_array().into_iter().flatten())
        .flat_map(|option| option.as_array().into_iter().flatten())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn contracts_of_a_real_archive_list_their_hauling_orders() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    let contracts = catalog["contracts"].as_array().ok_or("contracts")?;
    let subtypes: HashSet<&str> = catalog["commodities"]
        .as_array()
        .ok_or("commodities")?
        .iter()
        .filter_map(|commodity| commodity["subtypeGuid"].as_str())
        .collect();
    let mut sources: BTreeMap<&str, usize> = BTreeMap::new();
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    let mut unknown: HashSet<&str> = HashSet::new();
    let (mut hauling, mut resources) = (0_usize, 0_usize);
    for contract in contracts {
        let orders = contract["haulingOrders"]
            .as_array()
            .ok_or("haulingOrders")?;
        hauling += usize::from(!orders.is_empty());
        for order in orders {
            *sources
                .entry(order["source"].as_str().ok_or("source")?)
                .or_default() += 1;
            for item in items(order) {
                *kinds
                    .entry(item["kind"].as_str().ok_or("kind")?)
                    .or_default() += 1;
                if let Some(resource) = item["resourceGuid"].as_str() {
                    resources += 1;
                    if !subtypes.contains(resource) {
                        unknown.insert(resource);
                    }
                }
            }
        }
    }
    print_line(&format!(
        "{hauling} of {} contracts haul; orders by source {sources:?}; items by kind {kinds:?}; \
         {resources} resource items, {} resources without a commodity",
        contracts.len(),
        unknown.len()
    ));
    assert!(hauling > 400, "contracts that haul: {hauling}");
    assert!(
        unknown.len() < 40,
        "resources without a commodity: {}",
        unknown.len()
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_cargo_manifests_of_a_real_archive_name_commodities() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    let subtypes: HashSet<&str> = catalog["commodities"]
        .as_array()
        .ok_or("commodities")?
        .iter()
        .filter_map(|commodity| commodity["subtypeGuid"].as_str())
        .collect();
    let manifests = catalog["cargoManifests"]
        .as_array()
        .ok_or("cargoManifests")?;
    let mut fills: BTreeMap<&str, usize> = BTreeMap::new();
    let mut unknown: HashSet<&str> = HashSet::new();
    let (mut resources, mut rules) = (0_usize, 0_usize);
    for manifest in manifests {
        *fills
            .entry(manifest["fillType"].as_str().ok_or("fillType")?)
            .or_default() += 1;
        rules += manifest["spawnRules"].as_array().map_or(0, Vec::len);
        for resource in manifest["resources"].as_array().ok_or("resources")? {
            resources += 1;
            if let Some(guid) = resource["resourceGuid"].as_str()
                && !subtypes.contains(guid)
            {
                unknown.insert(guid);
            }
        }
    }
    print_line(&format!(
        "{} cargo manifests, fills {fills:?}, {resources} resources ({} resources \
         without a commodity), {rules} spawn rules",
        manifests.len(),
        unknown.len()
    ));
    assert!(manifests.len() > 60, "manifests: {}", manifests.len());
    assert!(
        unknown.len() < 10,
        "resources without a commodity: {}",
        unknown.len()
    );
    Ok(())
}
