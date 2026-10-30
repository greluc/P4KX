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

/// A bound far above any cooldown the game states, whatever its unit.
const MAX_SANE_TIME: f64 = 1_000_000.0;

#[expect(
    clippy::print_stdout,
    reason = "the figures are the result of the verification"
)]
fn print_line(line: &str) {
    println!("{line}");
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn contracts_of_a_real_archive_carry_a_cooldown() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    let contracts = catalog["contracts"].as_array().ok_or("contracts")?;
    let stated: Vec<&Value> = contracts
        .iter()
        .filter(|entry| !entry["cooldownLayer"].is_null())
        .collect();
    print_line(&format!(
        "{} of {} contracts state a cooldown",
        stated.len(),
        contracts.len()
    ));
    assert_eq!(
        stated.len(),
        contracts.len(),
        "every contract has a cooldown"
    );
    let mut longest: Option<(&Value, f64)> = None;
    for entry in &stated {
        let time = entry["personalCooldownTime"]
            .as_f64()
            .ok_or("personalCooldownTime is a number")?;
        assert!(
            (0.0..=MAX_SANE_TIME).contains(&time),
            "personal time {time}"
        );
        if longest.is_none_or(|(_, best)| time > best) {
            longest = Some((entry, time));
        }
    }
    let (sample, _) = longest.ok_or("a contract with a cooldown")?;
    print_line(&format!("cooldown sample: {sample}"));
    assert_eq!(sample["hasPersonalCooldown"], true);
    Ok(())
}
