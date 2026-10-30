// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Contracts by their `id`: `timeToComplete`, `contractBuyInAmount` and the minimum standing;
//! and the `minReputation` of every `SReputationStandingParams` a contract names.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use p4kx_extraction::Guid;
use serde_json::Value;

use super::{Findings, P4kx, elements, reference_guid};

const STANDINGS: &str = "standings";

/// The SCMDB lists of contracts and the category each is reported under.
const LISTS: [(&str, &str); 2] = [
    ("contracts", "contracts"),
    ("legacyContracts", "legacy contracts"),
];

/// The SCMDB fields of a contract that name a standing Record.
const STANDING_SIDES: [&str; 2] = ["minStanding", "maxStanding"];

/// The compared fields of one contract of a contract Record.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct ContractFields {
    time_to_complete: Option<f64>,
    buy_in: Option<f64>,
    /// The GUID `minStanding` links to; `None` when the field is absent or a null Reference.
    min_standing: Option<String>,
}

impl ContractFields {
    fn of(contract: &serde_json::Map<String, Value>) -> Self {
        let results = contract.get("contractResults").unwrap_or(&Value::Null);
        Self {
            time_to_complete: results["timeToComplete"].as_f64(),
            buy_in: results["contractBuyInAmount"].as_f64(),
            min_standing: contract
                .get("minStanding")
                .and_then(reference_guid)
                .map(str::to_owned),
        }
    }
}

/// Adds every object with a GUID `id` in the Record JSON `record` to `contracts`; a sub-contract
/// is such an object too and has none of the compared fields.
pub(crate) fn collect_contracts(record: &Value, contracts: &mut HashMap<String, ContractFields>) {
    match record {
        Value::Object(object) => {
            if let Some(id) = object.get("id").and_then(Value::as_str)
                && id.parse::<Guid>().is_ok()
            {
                contracts
                    .entry(id.to_owned())
                    .or_insert_with(|| ContractFields::of(object));
            }
            object
                .values()
                .for_each(|value| collect_contracts(value, contracts));
        }
        Value::Array(values) => values
            .iter()
            .for_each(|value| collect_contracts(value, contracts)),
        _ => {}
    }
}

pub(super) fn record_guids<'d>(merged: &'d Value, guids: &mut BTreeSet<&'d str>) {
    guids.extend(standings(merged).into_keys());
}

pub(super) fn compare(merged: &Value, p4kx: &P4kx, findings: &mut Findings) {
    for (list, category) in LISTS {
        for scmdb in elements(&merged[list]) {
            compare_contract(category, scmdb, &p4kx.contracts, findings);
        }
    }
    for (guid, scmdb) in standings(merged) {
        let ident = scmdb["name"].as_str().unwrap_or(guid);
        let record = p4kx.records.get(guid);
        findings.check(STANDINGS, record.is_some(), || {
            format!("{ident}: {guid} is no Record")
        });
        if let Some(record) = record {
            findings.number(
                STANDINGS,
                ident,
                "minReputation",
                scmdb["minReputation"].as_f64(),
                record["minReputation"].as_f64(),
            );
        }
    }
}

fn compare_contract(
    category: &'static str,
    scmdb: &Value,
    contracts: &HashMap<String, ContractFields>,
    findings: &mut Findings,
) {
    let id = scmdb["id"].as_str().unwrap_or_default();
    let ident = scmdb["debugName"].as_str().unwrap_or(id);
    let contract = contracts.get(id);
    findings.check(category, contract.is_some(), || {
        format!("{ident}: id {id} is in no contract Record")
    });
    let Some(contract) = contract else { return };
    findings.number_if_both(
        category,
        ident,
        "timeToComplete",
        scmdb["timeToComplete"].as_f64(),
        contract.time_to_complete,
    );
    findings.number_if_both(
        category,
        ident,
        "buyIn",
        scmdb["buyIn"].as_f64(),
        contract.buy_in,
    );
    findings.text_if_both(
        category,
        ident,
        "minStanding",
        scmdb["minStanding"]["guid"].as_str(),
        contract.min_standing.as_deref(),
    );
}

/// Returns every standing a contract of `merged` names, by its GUID.
fn standings(merged: &Value) -> BTreeMap<&str, &Value> {
    LISTS
        .iter()
        .flat_map(|(list, _)| elements(&merged[*list]))
        .flat_map(|contract| STANDING_SIDES.map(|side| &contract[side]))
        .filter_map(|standing| Some((standing["guid"].as_str()?, standing)))
        .collect()
}
