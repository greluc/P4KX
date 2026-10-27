// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{DataForge, Filter, RecordTree, StructValue, TreeBuilder, Value};

use crate::contract_recovery::{EscalatingClaims, ItemRecovery, RecoveryOverride};
use crate::guid_text::CatalogGuid;
use crate::record_fields::{class_name, elements, integer, single, structure};

const TYPE_NAME: &str = "ItemRecoveryConfigurationParams";

/// Returns the item recovery rules of `forge`: the `ItemRecoveryConfigurationParams` Record with
/// the smallest record path that `filter` matches and that can be read, `None` when there is none.
pub(crate) fn item_recovery<'f>(
    forge: &'f DataForge<'f>,
    filter: Option<&Filter>,
) -> Option<ItemRecovery<'f>> {
    let mut trees = TreeBuilder::new();
    let mut best: Option<RecordTree<'f>> = None;
    for record in forge.records() {
        if record.type_name().ok() != Some(TYPE_NAME) {
            continue;
        }
        let Ok(path) = record.record_path() else {
            continue;
        };
        if filter.is_some_and(|filter| !filter.matches(path)) {
            continue;
        }
        let Ok(tree) = trees.build(&record) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|have| tree.record_path() < have.record_path())
        {
            best = Some(tree);
        }
    }
    best.map(|tree| recovery(&tree))
}

fn recovery<'f>(tree: &RecordTree<'f>) -> ItemRecovery<'f> {
    let economy = structure(Some(tree.root()), "economyParams");
    let number = |name: &str| single(economy, name);
    ItemRecovery {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        global_brick_timer: number("globalBrickTimer"),
        base_reclaim_time: number("baseReclaimTime"),
        base_delivery_time: number("baseDeliveryTime"),
        scaling_price_floor: number("scalingPriceFloor"),
        a_uec_per_second: number("aUECperSecond"),
        default_loadout_cooldown_multiplier: number("defaultLoadoutCooldownMultiplier"),
        default_loadout_delivery_time_multiplier: number("defaultLoadoutDeliveryTimeMultiplier"),
        claim_cost_multiplier: number("claimCostMultiplier"),
        vehicle_chassis_repair_cost_multiplier: number("vehicleChassisRepairCostMultiplier"),
        vehicle_component_repair_cost_multiplier: number("vehicleComponentRepairCostMultiplier"),
        vehicle_repair_missing_item_cost_multiplier: number(
            "vehicleRepairMissingItemCostMultiplier",
        ),
        delivery_seconds_per_auec: number("deliverySecondsPerAUEC"),
        expedited_delivery_seconds_per_auec: number("expeditedDeliverySecondsPerAUEC"),
        default_loadout_expedited_delivery_time_multiplier: number(
            "defaultLoadoutExpeditedDeliveryTimeMultiplier",
        ),
        flat_minimum_delivery_timer_seconds: number("flatMinimumDeliveryTimerSeconds"),
        max_expedited_claim_cost_multiplier: number("maxExpeditedClaimCostMultiplier"),
        escalating_claims_params: structure(economy, "escalatingClaimsParams").map(escalating),
        overrides: elements(economy, "overrides")
            .iter()
            .filter_map(|group| match group {
                Value::Struct(group) => Some(group_override(group)),
                _ => None,
            })
            .collect(),
    }
}

fn escalating(params: &StructValue<'_>) -> EscalatingClaims {
    let params = Some(params);
    EscalatingClaims {
        escalating_claim_chain_limit: integer(params, "escalatingClaimChainLimit"),
        escalation_timer_multiplier: single(params, "escalationTimerMultiplier"),
        escalating_claim_cost_multiplier: single(params, "escalatingClaimCostMultiplier"),
        escalating_delivery_timer_multiplier: single(params, "escalatingDeliveryTimerMultiplier"),
    }
}

fn group_override(group: &StructValue<'_>) -> RecoveryOverride {
    let group = Some(group);
    let number = |name: &str| single(group, name);
    RecoveryOverride {
        class_guids: elements(group, "classes")
            .iter()
            .filter_map(|class| match class {
                Value::Reference(reference) if reference.location().is_none() => {
                    Some(CatalogGuid(reference.guid()))
                }
                _ => None,
            })
            .collect(),
        claim_cooldown_multiplier: number("claimCooldownMultiplier"),
        claim_cost_multiplier: number("claimCostMultiplier"),
        delivery_time_multiplier: number("deliveryTimeMultiplier"),
        max_expedite_cost_multiplier: number("maxExpediteCostMultiplier"),
        expedited_delivery_time_multiplier: number("expeditedDeliveryTimeMultiplier"),
        flat_minimum_delivery_timer_seconds_multiplier: number(
            "flatMinimumDeliveryTimerSecondsMultiplier",
        ),
        escalation_timer_multiplier: number("escalationTimerMultiplier"),
        escalating_claim_cost_multiplier: number("escalatingClaimCostMultiplier"),
        escalating_delivery_timer_multiplier: number("escalatingDeliveryTimerMultiplier"),
    }
}
