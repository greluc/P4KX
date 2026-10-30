// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{
    DataForge, DataForgeError, Filter, Record, RecordTree, StructValue, TreeBuilder, Value,
};

use crate::CatalogFailure;
use crate::contract_recovery::{EscalatingClaims, ItemRecovery, RecoveryOverride};
use crate::guid_text::CatalogGuid;
use crate::record_fields::{class_name, elements, integer, number, record_reference, structure};
use crate::selection::KindRule;

/// Selects the Records that hold the item recovery rules.
const RECOVERY: KindRule = KindRule {
    prefixes: &["libs/foundry/records/entitlementpolicies/"],
    type_name: "ItemRecoveryConfigurationParams",
};

/// The item recovery rules of a DataForge and the Records that could not be read.
pub(crate) struct Recovery<'f> {
    /// The rules of the Record with the smallest record path that can be read, `None` when there
    /// is none.
    pub(crate) rules: Option<ItemRecovery<'f>>,
    /// The selected Records that could not be read; they are left out.
    pub(crate) failures: Vec<CatalogFailure>,
}

/// Reads the item recovery rules of `forge` from the `ItemRecoveryConfigurationParams` Records
/// that `filter` matches.
pub(crate) fn read_recovery<'f>(forge: &'f DataForge<'f>, filter: Option<&Filter>) -> Recovery<'f> {
    let mut trees = TreeBuilder::new();
    let mut best: Option<RecordTree<'f>> = None;
    let mut failures = Vec::new();
    for record in forge.records() {
        match is_selected(&record, filter) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(source) => {
                failures.push(CatalogFailure::of(&record, &source));
                continue;
            }
        }
        match trees.build(&record) {
            Ok(tree) => {
                if best
                    .as_ref()
                    .is_none_or(|have| tree.record_path() < have.record_path())
                {
                    best = Some(tree);
                }
            }
            Err(source) => failures.push(CatalogFailure::of(&record, &source)),
        }
    }
    Recovery {
        rules: best.map(|tree| recovery(&tree)),
        failures,
    }
}

fn is_selected(record: &Record<'_>, filter: Option<&Filter>) -> Result<bool, DataForgeError> {
    let path = record.record_path()?;
    Ok(RECOVERY.matches_path(path)
        && filter.is_none_or(|filter| filter.matches(path))
        && record.type_name()? == RECOVERY.type_name)
}

fn recovery<'f>(tree: &RecordTree<'f>) -> ItemRecovery<'f> {
    let economy = structure(Some(tree.root()), "economyParams");
    let number = |name: &str| number(economy, name);
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
        escalation_timer_multiplier: number(params, "escalationTimerMultiplier"),
        escalating_claim_cost_multiplier: number(params, "escalatingClaimCostMultiplier"),
        escalating_delivery_timer_multiplier: number(params, "escalatingDeliveryTimerMultiplier"),
    }
}

fn group_override(group: &StructValue<'_>) -> RecoveryOverride {
    let group = Some(group);
    let number = |name: &str| number(group, name);
    RecoveryOverride {
        class_guids: elements(group, "classes")
            .iter()
            .filter_map(record_reference)
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
