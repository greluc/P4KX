// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The cooldown of contracts in the Basetool catalog when parameter overrides state it.

use serde_json::json;

use crate::catalog_contracts::{TestResult, entry};
use crate::catalog_contracts_facts::{BARE, OVERRIDDEN, SUB, SUB_DIRECT, assert_fields, catalog};

#[test]
fn a_handler_default_is_stated_by_the_handler() -> TestResult {
    let catalog = catalog()?;
    assert_fields(
        entry(&catalog, BARE, None)?,
        &json!({
            "cooldownLayer": "handler",
            "hasPersonalCooldown": true,
            "personalCooldownTime": 120.0,
            "personalCooldownTimeVariation": null,
            "cooldownLayers": {
                "hasPersonalCooldown": "handler",
                "personalCooldownTime": "handler",
                "personalCooldownTimeVariation": null,
                "abandonedCooldownTime": "handler",
                "abandonedCooldownTimeVariation": null,
            },
        }),
    );
    Ok(())
}

#[test]
fn a_handler_parameter_wins_over_the_default_availability_of_the_cooldown() -> TestResult {
    let catalog = catalog()?;
    assert_eq!(
        entry(&catalog, BARE, None)?["abandonedCooldownTime"],
        json!(15.0)
    );
    Ok(())
}

#[test]
fn a_contract_override_wins_and_names_the_contract_as_the_layer() -> TestResult {
    let catalog = catalog()?;
    assert_fields(
        entry(&catalog, OVERRIDDEN, None)?,
        &json!({
            "cooldownLayer": "contract",
            "hasPersonalCooldown": false,
            "personalCooldownTime": 45.0,
            "abandonedCooldownTime": 15.0,
            "cooldownLayers": {
                "hasPersonalCooldown": "contract",
                "personalCooldownTime": "contract",
                "personalCooldownTimeVariation": null,
                "abandonedCooldownTime": "handler",
                "abandonedCooldownTimeVariation": null,
            },
        }),
    );
    Ok(())
}

#[test]
fn a_float_override_of_a_sub_contract_matches_the_name_without_regard_to_case() -> TestResult {
    let catalog = catalog()?;
    assert_fields(
        entry(&catalog, OVERRIDDEN, Some(SUB))?,
        &json!({
            "cooldownLayer": "subContract",
            "hasPersonalCooldown": false,
            "personalCooldownTime": 7.5,
            "abandonedCooldownTime": 15.0,
            "cooldownLayers": {
                "hasPersonalCooldown": "contract",
                "personalCooldownTime": "subContract",
                "personalCooldownTimeVariation": null,
                "abandonedCooldownTime": "handler",
                "abandonedCooldownTimeVariation": null,
            },
        }),
    );
    Ok(())
}

#[test]
fn a_sub_contract_without_overrides_follows_its_contract() -> TestResult {
    let catalog = catalog()?;
    assert_fields(
        entry(&catalog, OVERRIDDEN, Some(SUB_DIRECT))?,
        &json!({"cooldownLayer": "contract", "personalCooldownTime": 45.0}),
    );
    Ok(())
}
