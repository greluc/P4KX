// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The links of the ship spawns and location queries of contracts in the Basetool catalog of a
//! real Archive.

use std::collections::HashSet;

use serde_json::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The contracts whose location prerequisite names a variable no location query states: the four
/// PVP bounties of North Rock, whose `MissionLocation` is not a location property.
const KNOWN_DANGLING: &str = "NorthRock_Bounty";

fn guids<'c>(catalog: &'c Value, kind: &str) -> Result<HashSet<&'c str>, String> {
    Ok(catalog[kind]
        .as_array()
        .ok_or_else(|| format!("{kind} is a list"))?
        .iter()
        .filter_map(|entry| entry["guid"].as_str())
        .collect())
}

/// Asserts that every ship spawn link and location query link of a contract names an entry of
/// its kind and that every location prerequisite names a variable of the contract's
/// `locationQueries`, apart from the known exceptions.
pub(crate) fn check_spawn_links(catalog: &Value) -> TestResult {
    let pools = guids(catalog, "contractShipSpawns")?;
    let queries = guids(catalog, "contractLocationQueries")?;
    for contract in catalog["contracts"].as_array().ok_or("contracts")? {
        for link in contract["shipSpawns"].as_array().ok_or("shipSpawns")? {
            let guid = link["poolGuid"].as_str().ok_or("poolGuid")?;
            assert!(pools.contains(guid), "ship spawn pool {guid}");
        }
        let mut variables = HashSet::new();
        for link in contract["locationQueries"]
            .as_array()
            .ok_or("locationQueries")?
        {
            let guid = link["queryGuid"].as_str().ok_or("queryGuid")?;
            assert!(queries.contains(guid), "location query {guid}");
            variables.insert(link["variable"].as_str().ok_or("variable")?);
        }
        for prerequisite in contract["locationPrerequisites"]
            .as_array()
            .ok_or("locationPrerequisites")?
        {
            let variable = prerequisite["variable"].as_str().ok_or("variable")?;
            let name = contract["debugName"].as_str().unwrap_or_default();
            assert!(
                variables.contains(variable) || name.starts_with(KNOWN_DANGLING),
                "{name}: location prerequisite {variable} names no location query"
            );
        }
    }
    Ok(())
}
