// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::StructValue;

use crate::blueprint::as_struct;
use crate::contract_offer::BlueprintReward;
use crate::mission_layers::{items, one};
use crate::mission_results::succeeds;
use crate::record_fields::{linked, number};

const BLUEPRINT_REWARDS: &str = "BlueprintRewards";

/// Returns the blueprint rewards that fire on success for the entry of `sub` in `contract`: those
/// of the sub contract when it states results of its own, else of the contract.
pub(crate) fn blueprint_rewards(
    contract: &StructValue<'_>,
    sub: Option<&StructValue<'_>>,
) -> Vec<BlueprintReward> {
    let own = sub
        .and_then(|sub| one(Some(sub), "contractResults"))
        .filter(|block| !items(Some(block), "contractResults").is_empty());
    let block = own.or_else(|| one(Some(contract), "contractResults"));
    items(block, "contractResults")
        .iter()
        .filter_map(as_struct)
        .filter(|result| result.type_name() == BLUEPRINT_REWARDS && succeeds(result))
        .filter_map(|result| {
            Some(BlueprintReward {
                pool_guid: linked(Some(result), "blueprintPool")?,
                chance: number(Some(result), "chance"),
            })
        })
        .collect()
}
