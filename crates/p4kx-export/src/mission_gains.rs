// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{StructValue, Value};

use crate::blueprint::as_struct;
use crate::contract_missions::ReputationGain;
use crate::guid_text::CatalogGuid;
use crate::mission_index::MissionIndex;
use crate::mission_layers::{items, one};
use crate::record_fields::{boolean, reference};

const LEGACY_REPUTATION: &str = "ContractResult_LegacyReputation";

/// Returns the reputation `contract` grants on success: one gain per amount of the results whose
/// first mission result is set.
pub(crate) fn gains<'f>(
    contract: &StructValue<'f>,
    index: &MissionIndex<'f>,
) -> Vec<ReputationGain<'f>> {
    let block = one(Some(contract), "contractResults");
    items(block, "contractResults")
        .iter()
        .filter_map(as_struct)
        .filter(|result| result.type_name() == LEGACY_REPUTATION && succeeds(result))
        .flat_map(|result| items(Some(result), "contractResultReputationAmounts"))
        .filter_map(as_struct)
        .map(|amount| {
            let faction = link(amount, "factionReputation");
            let scope = link(amount, "reputationScope");
            ReputationGain {
                faction_reputation_guid: faction.map(|(guid, _)| guid),
                faction_reputation_name: faction.map(|(_, name)| name),
                scope_guid: scope.map(|(guid, _)| guid),
                scope_name: scope.map(|(_, name)| name),
                amount: reference(Some(amount), "reward").and_then(|reward| index.reward(reward.0)),
            }
        })
        .collect()
}

/// Returns the GUID and the Record name the Reference in the field `name` of `parent` links to.
pub(crate) fn link<'f>(parent: &StructValue<'f>, name: &str) -> Option<(CatalogGuid, &'f str)> {
    match parent.field(name)? {
        Value::Reference(target) if target.location().is_none() => {
            let path = target.record_path();
            Some((
                CatalogGuid(target.guid()),
                path.rsplit('/').next().unwrap_or(path),
            ))
        }
        _ => None,
    }
}

fn succeeds(result: &StructValue<'_>) -> bool {
    match items(Some(result), "missionResults").first() {
        Some(Value::Bool(set)) => *set,
        Some(Value::Struct(flag)) => boolean(Some(flag), "value").unwrap_or(false),
        _ => false,
    }
}
