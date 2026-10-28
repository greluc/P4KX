// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The cooldown fields of contracts in the Basetool catalog.

use crate::catalog_contracts::{A, B, SUB_ONE, TestResult, build, entry};

use serde_json::{Value, json};

#[test]
fn a_contract_and_its_sub_contracts_carry_the_cooldown_of_their_handler() -> TestResult {
    let (_, catalog) = build(None)?;
    let stated = json!({
        "cooldownLayer": "handler",
        "hasPersonalCooldown": true,
        "personalCooldownTime": 2880.0,
        "personalCooldownTimeVariation": 60.0,
        "abandonedCooldownTime": 30.0,
        "abandonedCooldownTimeVariation": 0.5,
    });
    for (guid, sub) in [(A, None), (A, Some(SUB_ONE)), (B, None)] {
        let contract = entry(&catalog, guid, sub)?;
        for (field, value) in stated.as_object().into_iter().flatten() {
            assert_eq!(&contract[field], value, "{field} of {guid} {sub:?}");
        }
    }
    Ok(())
}

#[test]
fn a_handler_without_cooldown_fields_gives_null_and_no_layer() -> TestResult {
    let (_, catalog) = build(None)?;
    let bounty = catalog["contracts"]
        .as_array()
        .ok_or("contracts")?
        .iter()
        .find(|entry| entry["handler"] == "Bounty_Career")
        .ok_or("Bounty_Career")?;
    for field in [
        "cooldownLayer",
        "hasPersonalCooldown",
        "personalCooldownTime",
        "personalCooldownTimeVariation",
        "abandonedCooldownTime",
        "abandonedCooldownTimeVariation",
    ] {
        assert_eq!(bounty[field], Value::Null, "{field}");
    }
    Ok(())
}
