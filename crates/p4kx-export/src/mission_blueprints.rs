// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::StructValue;

use crate::blueprint::as_struct;
use crate::contract_offer::{BlueprintReward, BlueprintSource};
use crate::mission_layers::items;
use crate::mission_results::{results, succeeds};
use crate::record_fields::{boolean, linked, number};

const BLUEPRINT_REWARDS: &str = "BlueprintRewards";
const TEMPORARY_BLUEPRINT: &str = "MissionModifier_TemporaryBlueprint";

/// The parameter overrides of an entry's levels, nearest first: sub contract, contract, handler.
pub(crate) struct Overrides<'t, 'f> {
    pub(crate) sub: Option<&'t StructValue<'f>>,
    pub(crate) contract: Option<&'t StructValue<'f>>,
    pub(crate) handler: Option<&'t StructValue<'f>>,
}

/// Returns the blueprint pools an entry hands out: the `BlueprintRewards` of `block` (its resolved
/// results) that fire on success, then the enabled `MissionModifier_TemporaryBlueprint` of the
/// nearest level of `overrides` that states one.
pub(crate) fn blueprint_rewards(
    block: Option<&StructValue<'_>>,
    overrides: &Overrides<'_, '_>,
) -> Vec<BlueprintReward> {
    let rewards = results(block)
        .filter(|result| result.type_name() == BLUEPRINT_REWARDS && succeeds(result))
        .filter_map(|result| {
            Some(BlueprintReward {
                pool_guid: linked(Some(result), "blueprintPool")?,
                source: BlueprintSource::Result,
                chance: number(Some(result), "chance"),
            })
        });
    let modifiers = [overrides.sub, overrides.contract, overrides.handler]
        .into_iter()
        .map(temporary_blueprints)
        .find(|found| !found.is_empty())
        .unwrap_or_default();
    rewards.chain(modifiers).collect()
}

/// Returns the enabled temporary blueprint modifiers in the `modifierOverrides` of `params`.
fn temporary_blueprints(params: Option<&StructValue<'_>>) -> Vec<BlueprintReward> {
    items(params, "modifierOverrides")
        .iter()
        .filter_map(as_struct)
        .filter(|modifier| {
            modifier.type_name() == TEMPORARY_BLUEPRINT
                && boolean(Some(modifier), "enabled") != Some(false)
        })
        .filter_map(|modifier| {
            Some(BlueprintReward {
                pool_guid: linked(Some(modifier), "blueprintPool")?,
                source: BlueprintSource::TemporaryModifier,
                chance: None,
            })
        })
        .collect()
}
