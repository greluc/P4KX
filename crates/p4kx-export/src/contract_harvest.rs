// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::every_field_required;
use crate::guid_text::CatalogGuid;

/// A harvestable provider preset: a `HarvestableProviderPreset` Record under
/// `harvestable/providerpresets`, what a place can spawn. Which places use it is in
/// `harvestableLocations`.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct HarvestableProviderPreset<'a> {
    /// The GUID of the Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type, e.g. `HPP_Stanton1a`.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// The groups, from `harvestableGroups` in stored order.
    pub(crate) groups: Vec<HarvestableGroup<'a>>,
    /// The areas of the surface, from `areas` in stored order.
    pub(crate) areas: Vec<HarvestableArea<'a>>,
}

/// A group of harvestables of a provider preset.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct HarvestableGroup<'a> {
    /// `groupName`, e.g. `SpaceShip_Mineables`.
    pub(crate) group_name: Option<&'a str>,
    /// `groupProbability`.
    pub(crate) group_probability: Option<f32>,
    /// The harvestables of the group, from `harvestables` in stored order.
    pub(crate) harvestables: Vec<HarvestableElement<'a>>,
}

/// One `HarvestableElement` of a group.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct HarvestableElement<'a> {
    /// What the element spawns; `null` when the element names neither a harvestable nor an
    /// entity.
    pub(crate) harvestable: Option<HarvestableLink<'a>>,
    /// The GUID of the Record the Reference `harvestableSetup` links to.
    pub(crate) setup_guid: Option<CatalogGuid>,
    /// The GUID of the `HarvestableClusterPreset` Record the Reference `clustering` links to.
    pub(crate) clustering_guid: Option<CatalogGuid>,
    /// `relativeProbability`, from 0 to 900 on LIVE.
    pub(crate) relative_probability: Option<f32>,
    /// The GUIDs of the `Tag` Records of `geometries`, in stored order.
    pub(crate) geometry_tag_guids: Vec<CatalogGuid>,
}

/// What a harvestable spawns, followed from the `HarvestablePreset` Record to the entity and its
/// mining parameters.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct HarvestableLink<'a> {
    /// The GUID of the `HarvestablePreset` Record the Reference `harvestable` links to; `null`
    /// when the element names an entity only.
    pub(crate) preset_guid: Option<CatalogGuid>,
    /// The Record name of the `HarvestablePreset` without its type, e.g.
    /// `Mining_Common_Aluminum`; `null` for none or an unreadable Record.
    pub(crate) preset_class_name: Option<&'a str>,
    /// The GUID of the entity: `harvestableEntityClass` of the element, else `entityClass` of the
    /// preset.
    pub(crate) entity_guid: Option<CatalogGuid>,
    /// The GUID of the `MineableComposition` Record the entity's `MineableParams` link to; `null`
    /// for an entity that is not mineable.
    pub(crate) mineable_composition_guid: Option<CatalogGuid>,
    /// The GUID of the `MiningGlobalParams` Record the entity's `MineableParams` link to.
    pub(crate) mining_global_params_guid: Option<CatalogGuid>,
}

/// One area of a provider preset.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct HarvestableArea<'a> {
    /// `debugGroupName`, e.g. `Acidic`.
    pub(crate) name: Option<&'a str>,
    /// `globalModifier`.
    pub(crate) global_modifier: Option<f32>,
    /// `objectPresetPaths` of the area type, in stored order.
    pub(crate) object_preset_paths: Vec<&'a str>,
    /// The modifiers of the area, from `modifiers` in stored order.
    pub(crate) modifiers: Vec<AreaModifier>,
}

/// One modifier of an area.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct AreaModifier {
    /// The index in `groups` of the group the modified element is in; `null` when the weak
    /// pointer `harvestableElement` does not point into this Record.
    pub(crate) group_index: Option<u32>,
    /// The index in the group's `harvestables` of the modified element.
    pub(crate) harvestable_index: Option<u32>,
    /// `harvestableModifier`, from 0 to 2 on LIVE.
    pub(crate) modifier: Option<f32>,
    /// The GUIDs of the `Tag` Records of `geometries`, in stored order.
    pub(crate) geometry_tag_guids: Vec<CatalogGuid>,
}

/// A place a harvestable provider preset is used: a container whose `.soc` names the preset, with
/// the starmap object it hangs below.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct HarvestableLocation {
    /// The GUID of the `HarvestableProviderPreset` Record.
    pub(crate) preset_guid: CatalogGuid,
    /// The GUID of the `StarMapObject` the container hangs below: the nearest object at or above
    /// the container's Child element, a private mining point left out; `null` when none.
    pub(crate) starmap_object_guid: Option<CatalogGuid>,
    /// The folder of the star system below `Data/ObjectContainers/PU/system` that reaches the
    /// container.
    pub(crate) system: String,
    /// The Archive path of the container (`.socpak`).
    pub(crate) container: String,
}
