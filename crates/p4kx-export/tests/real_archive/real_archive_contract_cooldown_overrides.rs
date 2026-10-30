// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The cooldown layer of contracts in the Basetool catalog of a real Archive, checked against the
//! parameter overrides in the `ContractGenerator` Records; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one.

use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, build_catalog};
use p4kx_extraction::{Archive, DataForgeEntry, StructValue, TreeBuilder, Value};
use p4kx_testkit::{real_archive_key, real_archive_path};
use serde_json::Value as Json;

type TestResult = Result<(), Box<dyn Error>>;

/// `(record GUID, contract GUID, sub contract GUID or empty)`.
type EntryKey = (String, String, String);

const COOLDOWN_PARAMS: [&str; 5] = [
    "HasPersonalCooldown",
    "PersonalCooldownTime",
    "PersonalCooldownTimeVariation",
    "AbandonedCooldownTime",
    "AbandonedCooldownTimeVariation",
];
const OVERRIDE_ARRAYS: [&str; 3] = [
    "boolParamOverrides",
    "intParamOverrides",
    "floatParamOverrides",
];
const CONTAINERS: [&str; 4] = [
    "contracts",
    "introContracts",
    "legacyContracts",
    "PVPBountyContract",
];

#[expect(
    clippy::print_stdout,
    reason = "the figures are the result of the verification"
)]
fn print_line(line: &str) {
    println!("{line}");
}

/// The structs the field `field` of `parent` holds, as an array or as a single struct.
fn structs<'a>(
    parent: &'a StructValue<'_>,
    field: &str,
) -> impl Iterator<Item = &'a StructValue<'a>> {
    let items: &[Value<'_>] = match parent.field(field) {
        Some(Value::Array(items)) => items,
        Some(single @ Value::Struct(_)) => std::slice::from_ref(single),
        _ => &[],
    };
    items.iter().filter_map(|item| match item {
        Value::Struct(inner) => Some(inner),
        _ => None,
    })
}

/// Whether `owner` overrides a cooldown parameter, in its `paramOverrides` or, for a sub contract,
/// directly.
fn overrides_cooldown(owner: &StructValue<'_>) -> bool {
    let params: Vec<&StructValue<'_>> = structs(owner, "paramOverrides").collect();
    std::iter::once(owner).chain(params).any(|params| {
        OVERRIDE_ARRAYS.iter().any(|array| {
            structs(params, array).any(|param| {
                matches!(param.field("param"), Some(Value::Enum(name) | Value::String(name))
                    if COOLDOWN_PARAMS.iter().any(|known| known.eq_ignore_ascii_case(name)))
            })
        })
    })
}

/// The entries whose contract or sub contract overrides a cooldown parameter, with the layer
/// that has to be reported: `contract` or `subContract`.
fn overriding_entries(
    archive: &Archive,
) -> Result<HashMap<EntryKey, &'static str>, Box<dyn Error>> {
    let entry = DataForgeEntry::read(archive)?;
    let forge = entry.open()?;
    let mut trees = TreeBuilder::new();
    let mut expected = HashMap::new();
    for record in forge.records() {
        if !record
            .record_path()?
            .contains("contracts/contractgenerator")
            || record.type_name()? != "ContractGenerator"
        {
            continue;
        }
        let tree = trees.build(&record)?;
        let record_guid = tree.guid().to_string();
        for handler in structs(tree.root(), "generators") {
            for contract in CONTAINERS.iter().flat_map(|name| structs(handler, name)) {
                let Some(Value::Guid(id)) = contract.field("id") else {
                    continue;
                };
                let key = |sub: String| (record_guid.clone(), id.to_string(), sub);
                let own = overrides_cooldown(contract);
                if own {
                    expected.insert(key(String::new()), "contract");
                }
                for sub in structs(contract, "subContracts") {
                    let Some(Value::Guid(sub_id)) = sub.field("id") else {
                        continue;
                    };
                    if overrides_cooldown(sub) {
                        expected.insert(key(sub_id.to_string()), "subContract");
                    } else if own {
                        expected.insert(key(sub_id.to_string()), "contract");
                    }
                }
            }
        }
    }
    Ok(expected)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_contract_with_a_cooldown_override_reports_the_layer_of_that_override() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let expected = overriding_entries(&archive)?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Json = serde_json::from_slice(built.json())?;
    let mut census: BTreeMap<&str, usize> = BTreeMap::new();
    let mut checked = 0_usize;
    for entry in catalog["contracts"].as_array().ok_or("contracts")? {
        let layer = entry["cooldownLayer"].as_str().unwrap_or("none");
        *census.entry(layer).or_default() += 1;
        let key = (
            entry["recordGuid"].as_str().ok_or("recordGuid")?.to_owned(),
            entry["guid"].as_str().ok_or("guid")?.to_owned(),
            entry["subContractGuid"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        );
        if let Some(expected_layer) = expected.get(&key) {
            assert_eq!(layer, *expected_layer, "cooldownLayer of {key:?}");
            checked += 1;
        }
    }
    print_line(&format!(
        "cooldownLayer census: {census:?}; {checked} overriding entries checked"
    ));
    assert!(checked > 0, "some contract overrides a cooldown");
    Ok(())
}
