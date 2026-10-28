// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The older loot Records: `LootTable` and `LootArchetype`.

use p4kx_extraction::{RecordTree, StructValue};

use crate::contract_loot::QuantityRange;
use crate::contract_loot_legacy::{
    LegacyLootArchetype, LegacyLootEntry, LegacyLootTable, LegacyLootTableEntry,
    LegacySecondaryEntry, LegacySecondaryGroup, LegacySpawnWith,
};
use crate::guid_text::CatalogGuid;
use crate::record_fields::{
    class_name, integer, number, reference, references, structs, structure, text,
};

/// Returns the older loot table the Record tree `tree` describes.
pub(crate) fn legacy_table<'f>(tree: &RecordTree<'f>) -> LegacyLootTable<'f> {
    let root = Some(tree.root());
    LegacyLootTable {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        entries: structs(root, "lootArchetypes")
            .map(|entry| {
                let constraints = structure(Some(entry), "numberOfResultsConstraints");
                LegacyLootTableEntry {
                    weight: number(Some(entry), "weight"),
                    archetype_guid: reference(Some(entry), "archetype"),
                    min_results: integer(constraints, "minResults"),
                    max_results: integer(constraints, "maxResults"),
                }
            })
            .collect(),
    }
}

/// Returns the older loot archetype the Record tree `tree` describes.
pub(crate) fn legacy_archetype<'f>(tree: &RecordTree<'f>) -> LegacyLootArchetype<'f> {
    let root = Some(tree.root());
    LegacyLootArchetype {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        excluded_tag_guids: references(structure(root, "excludedTags"), "tags"),
        entries: structs(structure(root, "primaryOrGroup"), "entries")
            .map(primary_entry)
            .collect(),
        secondary_groups: structs(root, "secondaryOrGroups")
            .map(secondary_group)
            .collect(),
    }
}

fn primary_entry<'f>(entry: &StructValue<'f>) -> LegacyLootEntry<'f> {
    let value = Some(entry);
    let additional = structure(value, "additionalTags");
    let optional: Vec<&StructValue<'f>> = structs(value, "optionalData").collect();
    LegacyLootEntry {
        name: text(value, "name"),
        weight: number(value, "weight"),
        tag_guid: reference(value, "tag"),
        additional_positive_tag_guids: references(additional, "positiveTags"),
        additional_negative_tag_guids: references(additional, "negativeTags"),
        stack_size: optional
            .iter()
            .filter(|data| data.type_name() == "EntryOptionalData_StackSize")
            .find_map(|data| range(Some(data))),
        spawn_with: optional
            .iter()
            .filter(|data| data.type_name() == "EntryOptionalData_SpawnWith")
            .map(|data| spawn_with(data))
            .collect(),
    }
}

fn spawn_with<'f>(data: &StructValue<'f>) -> LegacySpawnWith<'f> {
    let value = Some(data);
    let term = structure(value, "tagsToMatch");
    LegacySpawnWith {
        name: text(value, "name"),
        mode: text(value, "mode"),
        min: number(value, "min"),
        max: number(value, "max"),
        positive_tag_guids: references(term, "positiveTags"),
        negative_tag_guids: references(term, "negativeTags"),
    }
}

fn range(data: Option<&StructValue<'_>>) -> Option<QuantityRange> {
    let min = number(data, "min");
    let max = number(data, "max");
    (min.is_some() || max.is_some()).then_some(QuantityRange { min, max })
}

fn secondary_group<'f>(group: &StructValue<'f>) -> LegacySecondaryGroup<'f> {
    LegacySecondaryGroup {
        group_name: text(Some(group), "groupName"),
        entries: structs(Some(group), "entries")
            .map(|entry| LegacySecondaryEntry {
                weight: number(Some(entry), "weight"),
                tag_guid: reference(Some(entry), "tag"),
            })
            .collect(),
    }
}
