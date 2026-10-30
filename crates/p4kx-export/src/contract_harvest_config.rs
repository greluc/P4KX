// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::every_field_required;
use crate::contract_harvest::HarvestableLink;
use crate::contract_loot::LootConfig;
use crate::guid_text::CatalogGuid;

/// A sub harvestable multi config: a `SubHarvestableMultiConfigRecord` Record under
/// `harvestable/slotpresets`, what the slots of an object can hold by tag.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct SubHarvestableMultiConfig<'a> {
    /// The GUID of the Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// The GUID of the Record the default config links to when it is a reference; `null` for none
    /// or a config of another form.
    pub(crate) default_config_guid: Option<CatalogGuid>,
    /// `ignoreAttachableTagsForTaggedConfigs`.
    pub(crate) ignore_attachable_tags: Option<bool>,
    /// The configs by tag, from `taggedConfigs` in stored order.
    pub(crate) tagged_configs: Vec<TaggedConfig<'a>>,
}

/// One config of a [`SubHarvestableMultiConfig`] and the tags it applies to.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct TaggedConfig<'a> {
    /// `name`.
    pub(crate) name: Option<&'a str>,
    /// The GUIDs of the `Tag` Records of the tag list, in stored order.
    pub(crate) tag_guids: Vec<CatalogGuid>,
    /// The GUID of the Record the config links to when it is a reference; `null` for a config
    /// written out in place.
    pub(crate) config_guid: Option<CatalogGuid>,
    /// `configRespawnTimeMultiplier` of a config written out in place.
    pub(crate) respawn_time_multiplier: Option<f32>,
    /// `initialSlotsProbability` of a config written out in place.
    pub(crate) initial_slots_probability: Option<f32>,
    /// The sub harvestables of a config written out in place, in stored order.
    pub(crate) sub_harvestables: Vec<SubHarvestable<'a>>,
}

/// One sub harvestable of a config.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct SubHarvestable<'a> {
    /// What the sub harvestable spawns; `null` when it names neither a harvestable nor an entity.
    pub(crate) harvestable: Option<HarvestableLink<'a>>,
    /// The GUID of the Record the Reference `harvestableSetup` links to.
    pub(crate) setup_guid: Option<CatalogGuid>,
    /// `harvestableRespawnTimeMultiplier`.
    pub(crate) respawn_time_multiplier: Option<f32>,
    /// `relativeProbability`.
    pub(crate) relative_probability: Option<f32>,
    /// `probability` of `relativeProbabilityDeepest`; `null` when there is none.
    pub(crate) relative_probability_deepest: Option<f32>,
    /// The GUIDs of the `Tag` Records of `geometries`, in stored order.
    pub(crate) geometry_tag_guids: Vec<CatalogGuid>,
    /// The loot the slot generates (`lootConfig`); `null` when the slot has none.
    pub(crate) loot_config: Option<LootConfig>,
}
