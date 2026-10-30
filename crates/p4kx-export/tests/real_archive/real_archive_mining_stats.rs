// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The mining laser, module and gadget stat blocks on a real Archive; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one.

use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, build_catalog};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

fn catalog() -> Result<Value, Box<dyn Error>> {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    Ok(serde_json::from_slice(built.json())?)
}

fn stats<'c>(catalog: &'c Value, class: &str) -> Result<&'c Value, String> {
    catalog["items"]
        .as_array()
        .and_then(|items| {
            items.iter().find(|item| {
                item["className"]
                    .as_str()
                    .is_some_and(|name| name.eq_ignore_ascii_case(class))
            })
        })
        .map(|item| &item["stats"])
        .ok_or_else(|| format!("no item {class}"))
}

fn count(catalog: &Value, present: impl Fn(&Value) -> bool) -> usize {
    catalog["items"].as_array().map_or(0, |items| {
        items.iter().filter(|item| present(&item["stats"])).count()
    })
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_ship_laser_states_its_throttle_modifiers_and_beams() -> TestResult {
    let catalog = catalog()?;
    let stats = stats(&catalog, "Mining_Laser_DRAK_Golem_S1")?;
    let laser = &stats["miningLaser"];
    assert_eq!(laser["throttleLerpSpeed"], 7.0);
    assert_eq!(laser["throttleMinimum"], 0.2);
    assert_eq!(laser["modifiers"]["instability"], 35.0);
    assert_eq!(laser["modifiers"]["optimalChargeWindowRate"], -40.0);
    assert!(laser["modifiers"]["shatterDamage"].is_null());
    assert_eq!(laser["filterModifier"], 40.0);
    assert!(laser["globalParamsGuid"].is_string());
    let beams = stats["weapon"]["fireActions"]
        .as_array()
        .ok_or("the fire actions")?;
    assert_eq!(beams[0]["hitType"], "ElectricArc");
    assert_eq!(beams[0]["fullDamageRange"], 40.0);
    assert_eq!(beams[0]["zeroDamageRange"], 45.0);
    assert_eq!(beams[0]["damagePerSecond"]["energy"], 3900.0);
    assert_eq!(beams[1]["hitType"], "Extraction");
    assert_eq!(beams[1]["damagePerSecond"]["energy"], 1295.0);
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn an_active_module_states_its_charges_lifetime_and_multipliers() -> TestResult {
    let catalog = catalog()?;
    let block = &stats(&catalog, "mining_modules_active_stampede")?["miningModifier"];
    assert_eq!(block["activationMethod"], "ActivateOnDemand");
    assert_eq!(block["charges"], 6);
    assert_eq!(block["laser"]["lifetime"], 30.0);
    assert_eq!(block["laser"]["modifiers"]["instability"], -10.0);
    assert_eq!(block["laser"]["modifiers"]["shatterDamage"], -10.0);
    let first = &block["fireActionModifiers"][0];
    assert_eq!(first["fireActionIndex"], 0);
    assert_eq!(first["damageMultiplier"], 1.35);
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_passive_module_and_a_gadget_state_their_modifiers() -> TestResult {
    let catalog = catalog()?;
    let passive = &stats(&catalog, "mining_modules_passive_vaux_mk1")?["miningModifier"];
    assert_eq!(passive["activationMethod"], "ActivateOnAttach");
    assert!(passive["laser"]["lifetime"].is_null());
    assert_eq!(
        passive["laser"]["modifiers"]["optimalChargeWindowRate"],
        -20.0
    );
    let gadget = &stats(&catalog, "mining_gadget_grin_optimax")?["miningModifier"];
    assert!(gadget["laser"].is_null());
    assert_eq!(
        gadget["rock"]["modifiers"]["optimalChargeWindowSize"],
        -30.0
    );
    assert_eq!(gadget["rock"]["modifiers"]["resistance"], -25.0);
    assert_eq!(gadget["rock"]["modifiers"]["clusterFactor"], 60.0);
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_catalog_lists_the_lasers_modules_and_gadgets_of_the_game() -> TestResult {
    let catalog = catalog()?;
    let lasers = count(&catalog, |stats| !stats["miningLaser"].is_null());
    let modules = count(&catalog, |stats| {
        !stats["miningModifier"]["laser"].is_null()
    });
    let gadgets = count(&catalog, |stats| !stats["miningModifier"]["rock"].is_null());
    assert!(lasers >= 18, "{lasers} mining lasers");
    assert!(modules >= 29, "{modules} modules");
    assert!(gadgets >= 6, "{gadgets} gadgets");
    Ok(())
}
