// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The ship insurance of the `ships` kind of the Basetool catalog of a real Archive; runs with
//! `--ignored` when `P4KX_DATA_P4K` names one.
//!
//! Run with `--nocapture` to see the counts. The anchor is the exact value of LIVE and follows
//! each patch; a failure after a patch means the value moved, to be checked against the Archive
//! and updated here and in `docs/basetool-catalog.md`.

use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, build_catalog};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

#[expect(clippy::print_stdout, reason = "the numbers are the result of the run")]
fn print_line(line: &str) {
    println!("{line}");
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_ships_of_a_real_archive_carry_their_insurance() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    let ships = catalog["ships"]
        .as_array()
        .ok_or("the catalog has no ships")?;
    let insured: Vec<&Value> = ships
        .iter()
        .filter(|ship| !ship["insurance"].is_null())
        .collect();
    let with_fee = insured
        .iter()
        .filter(|ship| ship["insurance"]["baseExpeditingFee"].is_u64())
        .count();
    print_line(&format!(
        "{} ships, {} with insurance, {} with an expediting fee",
        ships.len(),
        insured.len(),
        with_fee
    ));
    assert!(!insured.is_empty(), "no ship carries insurance");
    assert_eq!(with_fee, insured.len(), "every insurance states its fee");
    let gladius = ships
        .iter()
        .find(|ship| ship["className"] == "AEGS_Gladius")
        .ok_or("the catalog has no AEGS_Gladius")?;
    print_line(&format!("AEGS_Gladius: {}", gladius["insurance"]));
    assert_eq!(gladius["insurance"]["restoreOnClaim"], true);
    assert_eq!(gladius["insurance"]["shipEntityClassName"], "AEGS_Gladius");
    assert_eq!(
        gladius["insurance"]["baseWaitTimeMinutes"].as_f64(),
        Some(5.83)
    );
    assert_eq!(
        gladius["insurance"]["mandatoryWaitTimeMinutes"].as_f64(),
        Some(1.94)
    );
    assert_eq!(gladius["insurance"]["baseExpeditingFee"], 2335);
    assert_eq!(
        gladius["recordPath"], "libs/foundry/records/entities/spaceships/aegs_gladius",
        "the source path"
    );
    Ok(())
}
