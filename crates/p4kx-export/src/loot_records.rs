// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{RecordTree, StructValue};

use crate::contract_loot::{
    LootArchetype, LootConfig, LootPoolFilter, LootTable, LootTableEntry, PoolFilterInstance,
    PoolFilterNode,
};
use crate::guid_text::CatalogGuid;
use crate::loot_entry::{archetype_entries, optional_integer, optional_number, quantity};
use crate::record_fields::{
    class_name, integer, number, reference, references, structs, structure, text,
};

/// The deepest chain of filters inside filters that is followed.
const MAX_FILTER_DEPTH: usize = 16;

/// Returns the loot table the Record tree `tree` describes.
pub(crate) fn loot_table<'f>(tree: &RecordTree<'f>) -> LootTable<'f> {
    let table = structure(Some(tree.root()), "lootTable");
    LootTable {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        entries: structs(table, "lootArchetypes").map(table_entry).collect(),
    }
}

fn table_entry<'f>(entry: &StructValue<'f>) -> LootTableEntry<'f> {
    let value = Some(entry);
    let archetype = structure(value, "archetype");
    LootTableEntry {
        name: text(value, "name"),
        weight: number(value, "weight"),
        archetype_guid: reference(archetype, "lootArchetypeRecord"),
        archetype: archetype
            .filter(|inline| inline.field("entries").is_some())
            .map(|inline| archetype_entries(Some(inline))),
        priority_count: optional_integer(value, "priorityCount"),
        choice_limit: optional_integer(value, "choiceLimit"),
        dupe_limit: optional_integer(value, "dupeLimit"),
        chance_to_exist: optional_number(value, "chanceToExist"),
    }
}

/// Returns the loot archetype the Record tree `tree` describes.
pub(crate) fn loot_archetype<'f>(tree: &RecordTree<'f>) -> LootArchetype<'f> {
    let archetype = structure(Some(tree.root()), "lootArchetype");
    LootArchetype {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        entries: archetype_entries(archetype),
    }
}

/// Returns the pool filter the Record tree `tree` describes.
pub(crate) fn pool_filter<'f>(tree: &RecordTree<'f>) -> LootPoolFilter<'f> {
    LootPoolFilter {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        filter: filter_node(structure(Some(tree.root()), "filter"), 0),
    }
}

fn filter_node<'f>(node: Option<&StructValue<'f>>, depth: usize) -> Option<PoolFilterNode<'f>> {
    let node = node?;
    let value = Some(node);
    let term = structure(value, "tagFilter");
    Some(PoolFilterNode {
        node_type: node.type_name(),
        filter_guid: reference(value, "filterRecord"),
        positive_tag_guids: references(term, "positiveTags"),
        negative_tag_guids: references(term, "negativeTags"),
        filters: if depth < MAX_FILTER_DEPTH {
            structs(value, "filters")
                .map(|instance| PoolFilterInstance {
                    name: text(Some(instance), "name"),
                    mode: text(Some(instance), "mode"),
                    filter: filter_node(structure(Some(instance), "filter"), depth + 1)
                        .map(Box::new),
                })
                .collect()
        } else {
            Vec::new()
        },
    })
}

/// Returns the loot the struct `config` (a `LootConfig`) generates, `None` for no struct.
pub(crate) fn loot_config(config: Option<&StructValue<'_>>) -> Option<LootConfig> {
    let config = config?;
    let value = Some(config);
    let constraints = structure(value, "lootConstraints");
    Some(LootConfig {
        loot_table_guid: reference(value, "lootTable"),
        loot_table_v3_guid: reference(value, "lootTableV3"),
        pool_filter_guid: reference(constraints, "poolFilter"),
        total_results_limit: integer(constraints, "totalResultsLimit"),
        chance_to_generate: number(constraints, "chanceToGenerate"),
        chance_to_generate_additional_inventories: number(
            constraints,
            "chanceToGenerateAdditionalAttachedInventories",
        ),
        fullness: quantity(structure(constraints, "fullnessFactorRange")),
    })
}
