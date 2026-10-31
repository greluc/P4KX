// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{Guid, StructValue, Value};

use crate::blueprint::as_struct;
use crate::contract_missions::ReputationGain;
use crate::guid_text::CatalogGuid;
use crate::mission_index::MissionIndex;
use crate::mission_layers::{items, one};
use crate::record_fields::{boolean, integer, number, reference};

const LEGACY_REPUTATION: &str = "ContractResult_LegacyReputation";
const COMPLETION_TAGS: &str = "ContractResult_CompletionTags";

/// The `ContractResults` block of a contract: what the contract states about its own run.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct RunTerms {
    /// `timeToComplete`.
    pub(crate) time_to_complete: Option<f32>,
    /// `contractBuyInAmount`.
    pub(crate) buy_in_amount: Option<i64>,
}

impl RunTerms {
    /// Reads the terms of `contract`.
    pub(crate) fn read(contract: &StructValue<'_>) -> Self {
        let block = one(Some(contract), "contractResults");
        Self {
            time_to_complete: number(block, "timeToComplete"),
            buy_in_amount: integer(block, "contractBuyInAmount"),
        }
    }
}

/// Returns the `contractResults` block an entry reads: the sub contract's when it states results
/// of its own, else the contract's.
pub(crate) fn entry_results<'t, 'f>(
    contract: &'t StructValue<'f>,
    sub: Option<&'t StructValue<'f>>,
) -> Option<&'t StructValue<'f>> {
    sub.and_then(|sub| one(Some(sub), "contractResults"))
        .filter(|block| !items(Some(block), "contractResults").is_empty())
        .or_else(|| one(Some(contract), "contractResults"))
}

/// Returns the reputation the `contractResults` block `block` grants on success.
pub(crate) fn gains<'f>(
    block: Option<&StructValue<'f>>,
    index: &MissionIndex<'f>,
) -> Vec<ReputationGain<'f>> {
    results(block)
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

/// Returns the completion tags `contract` grants on success, in stored order.
pub(crate) fn granted_tags(contract: &StructValue<'_>) -> Vec<Guid> {
    results(one(Some(contract), "contractResults"))
        .filter(|result| result.type_name() == COMPLETION_TAGS && succeeds(result))
        .flat_map(|result| items(Some(result), "completionTags"))
        .filter_map(as_struct)
        .filter_map(|completion| reference(Some(completion), "tag"))
        .map(|tag| tag.0)
        .collect()
}

/// Returns the link in the field `name` of `parent`: the GUID and the Record name of its target.
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

/// Returns the results of the `contractResults` block `block`, in stored order.
pub(crate) fn results<'t, 'f>(
    block: Option<&'t StructValue<'f>>,
) -> impl Iterator<Item = &'t StructValue<'f>> {
    items(block, "contractResults").iter().filter_map(as_struct)
}

pub(crate) fn succeeds(result: &StructValue<'_>) -> bool {
    match items(Some(result), "missionResults").first() {
        Some(Value::Bool(set)) => *set,
        Some(Value::Struct(flag)) => boolean(Some(flag), "value").unwrap_or(false),
        _ => false,
    }
}
