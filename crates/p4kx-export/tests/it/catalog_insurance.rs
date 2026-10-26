// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The insurance data of the ships in the Basetool catalog, through the public API.

use crate::insurance_fixture::{BARE, INSURED, ITEM, UNINSURED, write_archive};

use std::error::Error;
use std::time::{Duration, UNIX_EPOCH};

use p4kx_export::{CatalogOptions, build_catalog};
use p4kx_extraction::Archive;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn build() -> Result<Value, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive =
        Archive::open_with_key(write_archive(dir.path())?, Some(p4kx_testkit::test_key()))?;
    let options = CatalogOptions::new(UNIX_EPOCH + Duration::from_secs(1_792_000_000));
    let built = build_catalog(&archive, &options)?;
    Ok(serde_json::from_slice(built.json())?)
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

fn ship(catalog: &Value, value: u128) -> Result<&Value, Box<dyn Error>> {
    let wanted = guid(value);
    catalog["ships"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|ship| ship["guid"] == wanted.as_str())
        .ok_or_else(|| format!("no ship {wanted}").into())
}

#[test]
fn a_ship_carries_the_claim_and_expedite_values_of_its_record() -> TestResult {
    let catalog = build()?;
    let insured = ship(&catalog, INSURED)?;
    assert_eq!(
        insured["insurance"],
        json!({
            "restoreOnClaim": true,
            "shipEntityClassName": "AEGS_Insured",
            "baseWaitTimeMinutes": 5.83,
            "mandatoryWaitTimeMinutes": 1.94,
            "expediteFee": 2335
        })
    );
    assert_eq!(
        insured["recordPath"], "libs/foundry/records/entities/spaceships/aegs_insured",
        "the source path is the ship's record path"
    );
    Ok(())
}

#[test]
fn a_ship_without_insurance_properties_has_null() -> TestResult {
    let catalog = build()?;
    assert_eq!(ship(&catalog, UNINSURED)?["insurance"], Value::Null);
    Ok(())
}

#[test]
fn empty_values_of_a_ground_vehicle_stay_facts() -> TestResult {
    let catalog = build()?;
    assert_eq!(
        ship(&catalog, BARE)?["insurance"],
        json!({
            "restoreOnClaim": false,
            "shipEntityClassName": null,
            "baseWaitTimeMinutes": 0.0,
            "mandatoryWaitTimeMinutes": 0.0,
            "expediteFee": 0
        })
    );
    Ok(())
}

#[test]
fn only_ships_carry_insurance() -> TestResult {
    let catalog = build()?;
    let wanted = guid(ITEM);
    let item = catalog["items"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|item| item["guid"] == wanted.as_str())
        .ok_or("the item is missing")?;
    assert!(item.get("insurance").is_none());
    Ok(())
}
