//! Contracts in the Basetool catalog of a real Archive; runs with `--ignored` when `P4KX_DATA_P4K`
//! names one.

use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, Kind, build_catalog};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLES: usize = 25;
const NAMED: [&str; 3] = ["Eckhart_Nyx_VLRT_DefendShip", "RCD_Contested", "Hockrow"];

fn text<'c>(entry: &'c Value, field: &str) -> &'c str {
    entry
        .get(field)
        .and_then(|text| text.get("en"))
        .and_then(Value::as_str)
        .unwrap_or_default()
}

#[expect(
    clippy::print_stdout,
    reason = "the figures are the result of the verification"
)]
fn gaps(contracts: &[Value]) {
    let describe = |label: &str, gap: &dyn Fn(&Value) -> bool| {
        let missing: Vec<&Value> = contracts.iter().filter(|entry| gap(entry)).collect();
        let no_template = missing
            .iter()
            .filter(|entry| entry["templateGuid"].is_null())
            .count();
        let mut by_handler: BTreeMap<&str, usize> = BTreeMap::new();
        let mut by_template: BTreeMap<&str, usize> = BTreeMap::new();
        for entry in &missing {
            *by_handler
                .entry(entry["handler"].as_str().unwrap_or_default())
                .or_default() += 1;
            *by_template
                .entry(entry["templateGuid"].as_str().unwrap_or("none"))
                .or_default() += 1;
        }
        println!(
            "{label}: {} entries, {no_template} without a template; handlers {:?}; templates {:?}",
            missing.len(),
            by_handler.iter().take(15).collect::<Vec<_>>(),
            by_template.iter().take(15).collect::<Vec<_>>()
        );
        for entry in missing.iter().take(8) {
            println!(
                "  {} | {} | {} | {}",
                entry["debugName"], entry["handler"], entry["templateGuid"], entry["recordPath"]
            );
        }
    };
    describe("untitled", &|entry| text(entry, "title").is_empty());
    describe("untyped", &|entry| entry["missionTypeGuid"].is_null());
    describe("unresolved tokens", &|entry| {
        let title = text(entry, "title");
        title.contains("[CONTRACTOR]") || title.contains("[RACETYPE]") || title.contains("[ITEM]")
    });
    describe("no pickup places", &|entry| {
        entry["pickupPlaces"].as_array().is_none_or(Vec::is_empty)
    });
    describe("negative gains", &|entry| {
        entry["reputationGains"].as_array().is_some_and(|gains| {
            gains
                .iter()
                .any(|gain| gain["amount"].as_i64().is_some_and(|amount| amount < 0))
        })
    });
}

#[expect(
    clippy::print_stdout,
    reason = "the figures are the result of the verification"
)]
fn report(contracts: &[Value]) {
    let subs = contracts
        .iter()
        .filter(|entry| !entry["subContractGuid"].is_null())
        .count();
    println!("{} entries, {subs} of them sub contracts", contracts.len());
    let mut placeholders: BTreeMap<&str, usize> = BTreeMap::new();
    let (mut untitled, mut untyped, mut gained, mut negative) = (0, 0, 0, 0);
    for entry in contracts {
        let title = text(entry, "title");
        untitled += usize::from(title.is_empty());
        untyped += usize::from(entry["missionTypeGuid"].is_null());
        let gains = entry["reputationGains"]
            .as_array()
            .map_or(&[][..], Vec::as_slice);
        gained += usize::from(!gains.is_empty());
        negative += gains
            .iter()
            .filter(|gain| gain["amount"].as_i64().is_some_and(|amount| amount < 0))
            .count();
        for part in title.split('[').skip(1) {
            if let Some((name, _)) = part.split_once(']') {
                *placeholders.entry(name).or_default() += 1;
            }
        }
    }
    println!("{untitled} without a title, {untyped} without a mission type");
    println!("{gained} with reputation gains, {negative} negative amounts among them");
    println!("placeholders in titles: {placeholders:?}");
    gaps(contracts);
    for entry in contracts.iter().take(SAMPLES) {
        println!(
            "{} | {:?} | {} | {} | {} | {}",
            entry["debugName"],
            entry["subContractGuid"],
            text(entry, "title"),
            text(entry, "missionType"),
            entry["pickupPlaces"],
            entry["reputationGains"]
        );
    }
    for name in NAMED {
        for entry in contracts
            .iter()
            .filter(|entry| {
                entry["debugName"]
                    .as_str()
                    .is_some_and(|debug| debug.contains(name))
            })
            .take(6)
        {
            println!(
                "{name}: {} | {:?} | {} | {}",
                entry["debugName"],
                entry["subContractGuid"],
                text(entry, "title"),
                entry["reputationGains"]
            );
        }
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the figures are the result of the verification"
)]
fn the_contracts_of_a_real_archive_resolve_titles_types_and_success_reputation() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    let contracts = catalog["contracts"].as_array().ok_or("contracts")?;
    report(contracts);
    let summary = built.summary().kind(Kind::Contracts);
    println!(
        "summary: {summary:?}, failures: {}",
        built.summary().failures().len()
    );

    assert!(!contracts.is_empty(), "contracts");
    assert_eq!(summary.map(|kind| kind.count), Some(contracts.len()));
    assert!(
        contracts
            .iter()
            .any(|entry| !entry["subContractGuid"].is_null()),
        "sub contracts"
    );
    let keys: HashSet<(&str, &str)> = contracts
        .iter()
        .map(|entry| {
            (
                entry["guid"].as_str().unwrap_or_default(),
                entry["subContractGuid"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        keys.len(),
        contracts.len(),
        "one entry per contract and sub contract"
    );
    assert!(
        contracts
            .iter()
            .all(|entry| !text(entry, "title").contains("~mission(")),
        "every token is replaced"
    );
    let untitled = contracts
        .iter()
        .filter(|entry| text(entry, "title").is_empty())
        .count();
    let untyped = contracts
        .iter()
        .filter(|entry| entry["missionTypeGuid"].is_null())
        .count();
    assert!(untitled * 100 < contracts.len(), "{untitled} untitled");
    assert!(untyped * 100 < contracts.len(), "{untyped} untyped");
    Ok(())
}
