// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::StructValue;

use crate::contract_loot::{LootArchetypeEntry, LootSelector, QuantityRange, SpawnWith};
use crate::record_fields::{integer, number, references, structs, structure, text};

/// Returns the entries of the archetype the struct `archetype` holds, in stored order.
pub(crate) fn archetype_entries<'f>(
    archetype: Option<&StructValue<'f>>,
) -> Vec<LootArchetypeEntry<'f>> {
    structs(archetype, "entries").map(entry).collect()
}

fn entry<'f>(entry: &StructValue<'f>) -> LootArchetypeEntry<'f> {
    let value = Some(entry);
    let optional: Vec<&StructValue<'f>> = structs(value, "optionalData").collect();
    LootArchetypeEntry {
        name: text(value, "name"),
        weight: number(value, "weight"),
        selector: selector(structure(value, "selector")),
        stack_size: optional
            .iter()
            .find_map(|data| quantity(structure(Some(data), "stackSize"))),
        spawn_with: optional
            .iter()
            .filter(|data| data.field("selector").is_some())
            .map(|data| spawn_with(data))
            .collect(),
    }
}

fn spawn_with<'f>(data: &StructValue<'f>) -> SpawnWith<'f> {
    let value = Some(data);
    SpawnWith {
        name: text(value, "name"),
        chance: number(value, "chanceToSpawnWith"),
        mode: text(value, "mode"),
        amount: quantity(structure(value, "amountToSpawn")),
        selector: selector(structure(value, "selector")),
    }
}

/// Returns what the struct `selector` selects: the tag term of its `tags`, the classes of its
/// manual `entityClasses` list.
fn selector<'f>(selector: Option<&StructValue<'f>>) -> LootSelector<'f> {
    let term = structure(selector, "tags");
    let classes = structure(selector, "entityClasses");
    LootSelector {
        selector_type: selector.map(StructValue::type_name),
        positive_tag_guids: references(term, "positiveTags"),
        negative_tag_guids: references(term, "negativeTags"),
        entity_class_guids: references(classes, "entityClasses"),
    }
}

/// Returns the range the struct `range` holds, `None` for none or an empty one.
pub(crate) fn quantity(range: Option<&StructValue<'_>>) -> Option<QuantityRange> {
    let min = number(range, "min");
    let max = number(range, "max");
    (min.is_some() || max.is_some()).then_some(QuantityRange { min, max })
}

/// Returns the first value of `field` among the `optionalData` elements of `entry`.
pub(crate) fn optional_number(entry: Option<&StructValue<'_>>, field: &str) -> Option<f32> {
    structs(entry, "optionalData").find_map(|data| number(Some(data), field))
}

/// Returns the first whole number of `field` among the `optionalData` elements of `entry`.
pub(crate) fn optional_integer(entry: Option<&StructValue<'_>>, field: &str) -> Option<i64> {
    structs(entry, "optionalData").find_map(|data| integer(Some(data), field))
}
