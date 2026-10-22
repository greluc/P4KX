//! The typed stat blocks of items in the Basetool catalog of a real Archive; runs with
//! `--ignored` when `P4KX_DATA_P4K` names one.

use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, build_catalog};
use p4kx_extraction::Archive;
use p4kx_testkit::real_archive_path;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

struct Block {
    name: &'static str,
    fields: &'static [&'static str],
    min_share: f64,
    required: bool,
}

const fn block(name: &'static str, fields: &'static [&'static str]) -> Block {
    Block {
        name,
        fields,
        min_share: 0.8,
        required: true,
    }
}

const BLOCKS: [Block; 9] = [
    Block {
        min_share: 0.4,
        ..block("weapon", &["ammoCapacity", "ammo"])
    },
    block("weapon", &["fireActions"]),
    block(
        "shield",
        &[
            "maxHealth",
            "maxRegen",
            "downedRegenDelay",
            "damagedRegenDelay",
        ],
    ),
    block(
        "quantumDrive",
        &["fuelRequirement", "jumpRange", "driveSpeed", "spoolUpTime"],
    ),
    block("powerPlant", &["powerOutput"]),
    block("cooler", &["coolingRate"]),
    block(
        "missile",
        &["damage", "explosionMaxRadius", "lockTime", "linearSpeed"],
    ),
    Block {
        required: false,
        ..block("power", &["powerBase", "powerDraw", "powerToEm"])
    },
    Block {
        required: false,
        ..block(
            "heat",
            &["thermalEnergyBase", "thermalEnergyDraw", "temperatureToIr"],
        )
    },
];

const fn present(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Array(items) => !items.is_empty(),
        _ => true,
    }
}

#[expect(clippy::print_stdout, reason = "the numbers are the result of the run")]
fn report(line: &str) {
    println!("{line}");
}

fn catalog() -> Result<Value, Box<dyn Error>> {
    let archive = Archive::open_with_key(real_archive_path()?, None)?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    report(&format!("catalog size: {} bytes", built.json().len()));
    Ok(serde_json::from_slice(built.json())?)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_stat_block_is_filled_for_the_items_that_have_it() -> TestResult {
    let catalog = catalog()?;
    let items = catalog["items"].as_array().ok_or("the items array")?;
    let with_stats = items.iter().filter(|item| !item["stats"].is_null()).count();
    report(&format!("{with_stats} of {} items have stats", items.len()));
    let mut problems = Vec::new();
    let state_holders = items
        .iter()
        .filter(|item| present(&item["stats"]["resourceStates"]))
        .count();
    report(&format!("resourceStates: {state_holders} items"));
    if state_holders == 0 {
        problems.push("no item has resource states".to_owned());
    }
    for Block {
        name: block,
        fields,
        min_share,
        required,
    } in BLOCKS
    {
        let holders: Vec<&Value> = items
            .iter()
            .filter(|item| !item["stats"][block].is_null())
            .collect();
        report(&format!("{block}: {} items", holders.len()));
        if holders.is_empty() {
            if required {
                problems.push(format!("no item has a {block} block"));
            }
            continue;
        }
        for field in fields {
            let filled = holders
                .iter()
                .filter(|item| present(&item["stats"][block][field]))
                .count();
            #[expect(clippy::cast_precision_loss, reason = "counts far below 2^52")]
            let share = filled as f64 / holders.len() as f64;
            report(&format!("  {field}: {filled} of {} filled", holders.len()));
            if share < min_share {
                problems.push(format!("{block}.{field}: {filled} of {}", holders.len()));
            }
        }
        if let Some(sample) = holders.iter().find(|item| {
            fields
                .iter()
                .all(|field| present(&item["stats"][block][field]))
        }) {
            report(&format!(
                "  sample {}: {}",
                sample["className"],
                serde_json::to_string(&sample["stats"])?
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "blocks below their share: {problems:?}"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn weapons_state_ammunition_damage_and_a_fire_rate() -> TestResult {
    let catalog = catalog()?;
    let items = catalog["items"].as_array().ok_or("the items array")?;
    let damaging = items
        .iter()
        .filter(|item| present(&item["stats"]["weapon"]["ammo"]["damage"]))
        .count();
    let rated = items
        .iter()
        .filter(|item| {
            item["stats"]["weapon"]["fireActions"]
                .as_array()
                .is_some_and(|actions| actions.iter().any(|action| present(&action["fireRate"])))
        })
        .count();
    report(&format!(
        "weapons with ammunition damage: {damaging}, with a fire rate: {rated}"
    ));
    assert!(damaging > 0, "a weapon with ammunition damage");
    assert!(rated > 0, "a weapon with a fire rate");
    Ok(())
}
