// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The contract facts in the Basetool catalog of a real Archive; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one.

use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, build_catalog};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const LAYERS: [&str; 4] = ["subContract", "contract", "handler", "template"];

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn contracts_of_a_real_archive_state_their_facts_and_the_level_of_each() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    let contracts = catalog["contracts"].as_array().ok_or("contracts")?;
    assert!(!contracts.is_empty(), "contracts");
    for entry in contracts {
        assert!(entry["timeToComplete"].is_number(), "timeToComplete");
        assert!(entry["buyInAmount"].is_number(), "buyInAmount");
        for field in [
            "onceOnly",
            "maxPlayersPerInstance",
            "availableInPrison",
            "canReacceptAfterAbandoning",
            "canReacceptAfterFailing",
            "hideInMobiGlas",
        ] {
            let layer = entry[field]["layer"].as_str().ok_or(field)?;
            assert!(LAYERS.contains(&layer), "{field}: {layer}");
        }
    }
    let resolved = contracts
        .iter()
        .flat_map(|entry| entry["requiredIntros"].as_array().into_iter().flatten())
        .any(|required| {
            required["introContracts"]
                .as_array()
                .is_some_and(|ids| !ids.is_empty())
        });
    assert!(resolved, "an intro contract grants a required tag");
    Ok(())
}
