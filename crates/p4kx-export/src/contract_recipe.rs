// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::every_field_required;
use crate::guid_text::CatalogGuid;

/// One tier of a blueprint: its craft time and its costs.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct BlueprintTier<'a> {
    /// The craft time of the tier in whole seconds, `null` without one.
    pub(crate) craft_time_seconds: Option<i64>,
    /// The mandatory cost, usually a `select`; `null` without one.
    pub(crate) mandatory: Option<CostLine<'a>>,
    /// The optional costs in stored order.
    pub(crate) optional: Vec<CostLine<'a>>,
}

/// One cost of a tier: a selection ("choose n") of costs, a resource line or an item line.
///
/// The fields of the other lines are `null`, or empty for `options` and `modifiers`.
#[derive(Debug, Default, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct CostLine<'a> {
    /// What the line is: `select`, `resource` or `item`. A closed set; a reader maps an unknown
    /// value to nothing.
    pub(crate) kind: &'static str,
    /// A `select`: how many options the player chooses, `null` when the Record holds no count.
    /// Fewer than `options` offers means the player may leave options out.
    pub(crate) count: Option<i64>,
    /// A `select`: the debug name of the slot, `null` without one.
    pub(crate) slot: Option<&'a str>,
    /// A `select`: the localisation key of the slot name, `null` without one or for a placeholder.
    pub(crate) slot_key: Option<&'a str>,
    /// A `select`: the options in stored order.
    pub(crate) options: Vec<Self>,
    /// A `resource`: the GUID of the `ResourceType` Record.
    pub(crate) resource_guid: Option<CatalogGuid>,
    /// An `item`: the GUID of the entity.
    pub(crate) item_guid: Option<CatalogGuid>,
    /// A `resource`: the quantity in standard cargo units (SCU), before the quantity multiplier.
    pub(crate) quantity_scu: Option<f32>,
    /// An `item`: the number of units, before the quantity multiplier.
    pub(crate) quantity_units: Option<i64>,
    /// A `resource` or `item`: the minimum quality, `null` without one.
    pub(crate) min_quality: Option<i64>,
    /// The `CraftingCostContext_QuantityMultiplier` factor of the line, `null` without one.
    pub(crate) quantity_multiplier: Option<f32>,
    /// The option of `CraftingCostContext_ResultCompositionInclusion`, e.g. `Include`; `null`
    /// without one. A closed set the game may extend.
    pub(crate) composition_inclusion: Option<&'a str>,
    /// The gameplay properties the line's quality scales
    /// (`CraftingCostContext_ResultGameplayPropertyModifiers`).
    pub(crate) modifiers: Vec<PropertyModifier>,
}

/// A gameplay property of the crafted item with the ranges that scale it by quality.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct PropertyModifier {
    /// The GUID of the `gameplayProperties` entry, `null` for a null Reference.
    pub(crate) property_guid: Option<CatalogGuid>,
    /// The ranges in stored order.
    pub(crate) ranges: Vec<ValueRange>,
}

/// One linear range of a modifier: the value at the quality `startQuality` and at `endQuality`.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct ValueRange {
    /// `multiplier` when the values are factors, `additive` when they are summands. A closed set;
    /// a reader maps an unknown value to nothing.
    pub(crate) kind: &'static str,
    /// The quality the range starts at, `null` when the Record holds none.
    pub(crate) start_quality: Option<i64>,
    /// The quality the range ends at, `null` when the Record holds none.
    pub(crate) end_quality: Option<i64>,
    /// The modifier at the start quality, `null` when the Record holds none.
    pub(crate) at_start: Option<f32>,
    /// The modifier at the end quality, `null` when the Record holds none.
    pub(crate) at_end: Option<f32>,
}

/// What dismantling the item of a blueprint returns: a link to the process and the tier.
///
/// The item returns the share `efficiency` of the process of the costs of the tier `tierIndex`,
/// except the resources in `craftingGlobal.dismantleExcludedResourceGuids`. A `select` is not
/// resolved and the quantity multiplier is not applied; these are assumptions, not verified
/// against the game.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct BlueprintDismantle {
    /// The GUID of the `dismantleProcesses` entry.
    pub(crate) process_guid: CatalogGuid,
    /// The index into `tiers` of the tier whose costs are returned, `null` for a blueprint
    /// without a tier.
    pub(crate) tier_index: Option<u32>,
}
