// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::every_field_required;
use crate::contract_loot::QuantityRange;
use crate::guid_text::CatalogGuid;

/// An older loot table: a `LootTable` Record under `lootgeneration/loottables`. The `lootTable`
/// of a slot's `lootConfig` links to one.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct LegacyLootTable<'a> {
    /// The GUID of the Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type, e.g. `Ammo`.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// The entries of `lootArchetypes` in stored order.
    pub(crate) entries: Vec<LegacyLootTableEntry>,
}

/// One entry of an older loot table: an archetype and the odds it is drawn.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct LegacyLootTableEntry {
    /// The `weight` of the entry among the entries of the table.
    pub(crate) weight: Option<f32>,
    /// The `LootArchetype` Record the entry links to, an entry of `legacyLootArchetypes`.
    pub(crate) archetype_guid: Option<CatalogGuid>,
    /// `minResults` of `numberOfResultsConstraints`.
    pub(crate) min_results: Option<i64>,
    /// `maxResults` of `numberOfResultsConstraints`.
    pub(crate) max_results: Option<i64>,
}

/// An older loot archetype: a `LootArchetype` Record under `lootgeneration/lootarchetypes`, the
/// weighted tags of item a table entry stands for.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct LegacyLootArchetype<'a> {
    /// The GUID of the Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type, e.g. `LootArchetype_Antiques_Common`.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// The `Tag` Records in `excludedTags`: items that carry one are never drawn.
    pub(crate) excluded_tag_guids: Vec<CatalogGuid>,
    /// The entries of `primaryOrGroup` in stored order.
    pub(crate) entries: Vec<LegacyLootEntry<'a>>,
    /// The groups of `secondaryOrGroups` in stored order.
    pub(crate) secondary_groups: Vec<LegacySecondaryGroup<'a>>,
}

/// One entry of the primary group of an older archetype.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct LegacyLootEntry<'a> {
    /// The designer's label of the entry.
    pub(crate) name: Option<&'a str>,
    /// The `weight` of the entry among the entries of the group.
    pub(crate) weight: Option<f32>,
    /// The `Tag` Record the entry draws items by.
    pub(crate) tag_guid: Option<CatalogGuid>,
    /// The `Tag` Records in `positiveTags` of `additionalTags`.
    pub(crate) additional_positive_tag_guids: Vec<CatalogGuid>,
    /// The `Tag` Records in `negativeTags` of `additionalTags`.
    pub(crate) additional_negative_tag_guids: Vec<CatalogGuid>,
    /// The `EntryOptionalData_StackSize` of the entry.
    pub(crate) stack_size: Option<QuantityRange>,
    /// The `EntryOptionalData_SpawnWith` items spawned along with the entry, in stored order.
    pub(crate) spawn_with: Vec<LegacySpawnWith<'a>>,
}

/// Items an older entry spawns along with its own (`EntryOptionalData_SpawnWith`).
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct LegacySpawnWith<'a> {
    /// The designer's label.
    pub(crate) name: Option<&'a str>,
    /// `mode`, e.g. `MostSimilar`.
    pub(crate) mode: Option<&'a str>,
    /// `min`, how many are spawned at least.
    pub(crate) min: Option<f32>,
    /// `max`, how many are spawned at most.
    pub(crate) max: Option<f32>,
    /// The `Tag` Records in `positiveTags` of `tagsToMatch`.
    pub(crate) positive_tag_guids: Vec<CatalogGuid>,
    /// The `Tag` Records in `negativeTags` of `tagsToMatch`.
    pub(crate) negative_tag_guids: Vec<CatalogGuid>,
}

/// A secondary group of an older archetype (`LootArchetypeOrGroup_Secondary`).
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct LegacySecondaryGroup<'a> {
    /// `groupName`, the designer's label, e.g. `Rarity`.
    pub(crate) group_name: Option<&'a str>,
    /// The entries of the group in stored order.
    pub(crate) entries: Vec<LegacySecondaryEntry>,
}

/// One entry of a secondary group.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct LegacySecondaryEntry {
    /// The `weight` of the entry among the entries of the group.
    pub(crate) weight: Option<f32>,
    /// The `Tag` Record of the entry.
    pub(crate) tag_guid: Option<CatalogGuid>,
}
