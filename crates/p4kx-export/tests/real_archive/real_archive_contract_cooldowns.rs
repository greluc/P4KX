// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The cooldown fields of contracts in the Basetool catalog of a real Archive; runs with
//! `--ignored` when `P4KX_DATA_P4K` names one.

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

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn contracts_of_a_real_archive_carry_the_cooldown_of_their_handler() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    let contracts = catalog["contracts"].as_array().ok_or("contracts")?;
    let stated: Vec<&Value> = contracts
        .iter()
        .filter(|entry| entry["cooldownLayer"] == "handler")
        .collect();
    print_line(&format!(
        "{} of {} contracts state a cooldown",
        stated.len(),
        contracts.len()
    ));
    assert!(stated.len() > 1_000, "most contracts have a cooldown");
    let long = stated
        .iter()
        .find(|entry| entry["personalCooldownTime"].as_f64() == Some(2880.0))
        .ok_or("a contract with a personal cooldown of 2880")?;
    print_line(&format!("cooldown sample: {long}"));
    assert_eq!(long["hasPersonalCooldown"], true);
    for entry in &stated {
        let time = entry["personalCooldownTime"].as_f64().unwrap_or_default();
        assert!((0.0..=2880.0).contains(&time), "personal time {time}");
    }
    Ok(())
}
