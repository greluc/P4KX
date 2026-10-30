// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::every_field_required;
use crate::guid_text::CatalogGuid;

/// The item recovery rules: an `ItemRecoveryConfigurationParams` Record, its `economyParams`.
///
/// The multipliers and rates of the Record under their data names. The Record holds no fee in
/// aUEC, so the catalog states none. Every number is that of the Record, `null` when the Record
/// states none or a value that is no number.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct ItemRecovery<'a> {
    /// The GUID of the Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type, e.g. `GlobalItemRecovery`.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// `globalBrickTimer`.
    pub(crate) global_brick_timer: Option<f32>,
    /// `baseReclaimTime`.
    pub(crate) base_reclaim_time: Option<f32>,
    /// `baseDeliveryTime`.
    pub(crate) base_delivery_time: Option<f32>,
    /// `scalingPriceFloor`.
    pub(crate) scaling_price_floor: Option<f32>,
    /// `aUECperSecond`.
    #[serde(rename = "aUECperSecond")]
    pub(crate) a_uec_per_second: Option<f32>,
    /// `defaultLoadoutCooldownMultiplier`.
    pub(crate) default_loadout_cooldown_multiplier: Option<f32>,
    /// `defaultLoadoutDeliveryTimeMultiplier`.
    pub(crate) default_loadout_delivery_time_multiplier: Option<f32>,
    /// `claimCostMultiplier`.
    pub(crate) claim_cost_multiplier: Option<f32>,
    /// `vehicleChassisRepairCostMultiplier`.
    pub(crate) vehicle_chassis_repair_cost_multiplier: Option<f32>,
    /// `vehicleComponentRepairCostMultiplier`.
    pub(crate) vehicle_component_repair_cost_multiplier: Option<f32>,
    /// `vehicleRepairMissingItemCostMultiplier`.
    pub(crate) vehicle_repair_missing_item_cost_multiplier: Option<f32>,
    /// `deliverySecondsPerAUEC`.
    #[serde(rename = "deliverySecondsPerAUEC")]
    pub(crate) delivery_seconds_per_auec: Option<f32>,
    /// `expeditedDeliverySecondsPerAUEC`.
    #[serde(rename = "expeditedDeliverySecondsPerAUEC")]
    pub(crate) expedited_delivery_seconds_per_auec: Option<f32>,
    /// `defaultLoadoutExpeditedDeliveryTimeMultiplier`.
    pub(crate) default_loadout_expedited_delivery_time_multiplier: Option<f32>,
    /// `flatMinimumDeliveryTimerSeconds`.
    pub(crate) flat_minimum_delivery_timer_seconds: Option<f32>,
    /// `maxExpeditedClaimCostMultiplier`.
    pub(crate) max_expedited_claim_cost_multiplier: Option<f32>,
    /// `escalatingClaimsParams`, `null` when the Record has none.
    pub(crate) escalating_claims_params: Option<EscalatingClaims>,
    /// The `overrides`: groups of entity classes whose multipliers replace the defaults, in the
    /// order of the Record.
    pub(crate) overrides: Vec<RecoveryOverride>,
}

/// The `escalatingClaimsParams` of the item recovery rules.
#[derive(Debug, Clone, Copy, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct EscalatingClaims {
    /// `escalatingClaimChainLimit`.
    pub(crate) escalating_claim_chain_limit: Option<i64>,
    /// `escalationTimerMultiplier`.
    pub(crate) escalation_timer_multiplier: Option<f32>,
    /// `escalatingClaimCostMultiplier`.
    pub(crate) escalating_claim_cost_multiplier: Option<f32>,
    /// `escalatingDeliveryTimerMultiplier`.
    pub(crate) escalating_delivery_timer_multiplier: Option<f32>,
}

/// An `ItemRecoveryOverrideGroupDef`: the multipliers of a group of entity classes.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct RecoveryOverride {
    /// The GUIDs of the entity classes the group applies to (`classes`), in the order of the
    /// Record. Each is the GUID of an `EntityClassDefinition` Record, an entry of `ships` for a
    /// ship; a Reference to no Record reads as null and is left out.
    pub(crate) class_guids: Vec<CatalogGuid>,
    /// `claimCooldownMultiplier`.
    pub(crate) claim_cooldown_multiplier: Option<f32>,
    /// `claimCostMultiplier`.
    pub(crate) claim_cost_multiplier: Option<f32>,
    /// `deliveryTimeMultiplier`.
    pub(crate) delivery_time_multiplier: Option<f32>,
    /// `maxExpediteCostMultiplier`.
    pub(crate) max_expedite_cost_multiplier: Option<f32>,
    /// `expeditedDeliveryTimeMultiplier`.
    pub(crate) expedited_delivery_time_multiplier: Option<f32>,
    /// `flatMinimumDeliveryTimerSecondsMultiplier`.
    pub(crate) flat_minimum_delivery_timer_seconds_multiplier: Option<f32>,
    /// `escalationTimerMultiplier`.
    pub(crate) escalation_timer_multiplier: Option<f32>,
    /// `escalatingClaimCostMultiplier`.
    pub(crate) escalating_claim_cost_multiplier: Option<f32>,
    /// `escalatingDeliveryTimerMultiplier`.
    pub(crate) escalating_delivery_timer_multiplier: Option<f32>,
}
