// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The contract facts in the Basetool catalog: time to complete, buy-in, sharing, legality,
//! availability and the required intro contracts.

use std::error::Error;
use std::time::{Duration, UNIX_EPOCH};

use p4kx_export::{CatalogOptions, build_catalog};
use p4kx_extraction::Archive;
use serde_json::{Value, json};

use crate::catalog_contracts::{TestResult, entry};
use crate::facts_fixture;

const BARE: &str = "21000000-0000-4000-8000-000000000001";
const OVERRIDDEN: &str = "21000000-0000-4000-8000-000000000002";
const INTRO: &str = "21000000-0000-4000-8000-000000000003";
const GATED: &str = "21000000-0000-4000-8000-000000000004";
const STRIPPED: &str = "21000000-0000-4000-8000-000000000006";
const SUB: &str = "22000000-0000-4000-8000-000000000001";
const SUB_DIRECT: &str = "22000000-0000-4000-8000-000000000002";
const TAG: &str = "23000000-0000-4000-8000-000000000001";
const FAIL_TAG: &str = "23000000-0000-4000-8000-000000000002";

fn catalog() -> Result<Value, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = Archive::open_with_key(
        facts_fixture::write_archive(dir.path())?,
        Some(p4kx_testkit::test_key()),
    )?;
    let options = CatalogOptions::new(UNIX_EPOCH + Duration::from_secs(1_792_000_000));
    Ok(serde_json::from_slice(
        build_catalog(&archive, &options)?.json(),
    )?)
}

fn at(value: impl Into<Value>, layer: &str) -> Value {
    json!({"value": value.into(), "layer": layer})
}

fn assert_fields(entry: &Value, expected: &Value) {
    for (field, value) in expected.as_object().into_iter().flatten() {
        assert_eq!(&entry[field], value, "{field}");
    }
}

#[test]
fn a_contract_without_overrides_takes_the_handler_and_the_template() -> TestResult {
    let catalog = catalog()?;
    assert_fields(
        entry(&catalog, BARE, None)?,
        &json!({
            "timeToComplete": 0.0,
            "buyInAmount": 0,
            "canBeShared": at(true, "template"),
            "illegal": at(false, "template"),
            "onceOnly": at(true, "handler"),
            "maxPlayersPerInstance": at(1, "handler"),
            "availableInPrison": at(false, "handler"),
            "canReacceptAfterAbandoning": at(true, "handler"),
            "canReacceptAfterFailing": at(false, "handler"),
            "hideInMobiGlas": at(true, "handler"),
        }),
    );
    Ok(())
}

#[test]
fn a_contract_override_wins_over_the_handler_and_the_template() -> TestResult {
    let catalog = catalog()?;
    assert_fields(
        entry(&catalog, OVERRIDDEN, None)?,
        &json!({
            "timeToComplete": 25.0,
            "buyInAmount": 1500,
            "canBeShared": at(true, "template"),
            "illegal": at(true, "contract"),
            "onceOnly": at(false, "contract"),
            "maxPlayersPerInstance": at(4, "contract"),
        }),
    );
    Ok(())
}

#[test]
fn a_sub_contract_override_wins_and_the_rest_follows_the_contract() -> TestResult {
    let catalog = catalog()?;
    assert_fields(
        entry(&catalog, OVERRIDDEN, Some(SUB))?,
        &json!({
            "timeToComplete": 25.0,
            "buyInAmount": 1500,
            "canBeShared": at(false, "subContract"),
            "illegal": at(true, "contract"),
            "onceOnly": at(false, "contract"),
        }),
    );
    Ok(())
}

#[test]
fn a_sub_contract_states_its_parameters_directly() -> TestResult {
    let catalog = catalog()?;
    assert_eq!(
        entry(&catalog, OVERRIDDEN, Some(SUB_DIRECT))?["onceOnly"],
        at(true, "subContract")
    );
    Ok(())
}

#[test]
fn a_required_tag_lists_the_intro_contracts_that_grant_it_on_success() -> TestResult {
    let catalog = catalog()?;
    assert_eq!(
        entry(&catalog, GATED, None)?["requiredIntros"],
        json!([
            {"level": "handler", "tagGuid": FAIL_TAG, "introContracts": []},
            {"level": "contract", "tagGuid": TAG, "introContracts": [INTRO]},
            {"level": "contract", "tagGuid": FAIL_TAG, "introContracts": []},
        ])
    );
    Ok(())
}

#[test]
fn a_contract_that_requires_nothing_inherits_the_handler_requirement_only() -> TestResult {
    let catalog = catalog()?;
    assert_eq!(
        entry(&catalog, BARE, None)?["requiredIntros"],
        json!([{"level": "handler", "tagGuid": FAIL_TAG, "introContracts": []}])
    );
    Ok(())
}

#[test]
fn a_sub_contract_lists_the_tags_it_requires_after_those_of_its_handler_and_contract() -> TestResult
{
    let catalog = catalog()?;
    assert_eq!(
        entry(&catalog, OVERRIDDEN, Some(SUB))?["requiredIntros"],
        json!([
            {"level": "handler", "tagGuid": FAIL_TAG, "introContracts": []},
            {"level": "subContract", "tagGuid": TAG, "introContracts": [INTRO]},
        ])
    );
    Ok(())
}

#[test]
fn a_handler_parameter_wins_over_its_default_availability() -> TestResult {
    let catalog = catalog()?;
    assert_eq!(
        entry(&catalog, BARE, None)?["hideInMobiGlas"],
        at(true, "handler")
    );
    Ok(())
}

#[test]
fn a_contract_without_template_results_or_availability_gives_null_everywhere() -> TestResult {
    let catalog = catalog()?;
    assert_fields(
        entry(&catalog, STRIPPED, None)?,
        &json!({
            "templateGuid": null,
            "timeToComplete": null,
            "buyInAmount": null,
            "canBeShared": null,
            "illegal": null,
            "onceOnly": null,
            "maxPlayersPerInstance": null,
            "availableInPrison": null,
            "canReacceptAfterAbandoning": null,
            "canReacceptAfterFailing": null,
            "hideInMobiGlas": null,
            "requiredIntros": [],
        }),
    );
    Ok(())
}
