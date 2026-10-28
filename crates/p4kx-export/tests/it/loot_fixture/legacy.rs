// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The older `LootTable` and `LootArchetype` Records of the loot fixture.

use p4kx_testkit::{TestInstance, TestRecord, TestValue};

use super::{array, instance, record};

const LOOT: &str = "libs/foundry/records/lootgeneration/";

/// The GUIDs the older Records link to or are known by.
pub(super) struct Ids {
    pub(super) table: u128,
    pub(super) archetype: u128,
    pub(super) tag_rare: u128,
    pub(super) tag_common: u128,
}

fn tags(guids: &[u128]) -> TestValue {
    TestValue::Array(guids.iter().copied().map(TestValue::Reference).collect())
}

fn term(positive: &[u128], negative: &[u128]) -> TestValue {
    TestValue::Struct(
        instance("TagsDNFTerm")
            .value("positiveTags", tags(positive))
            .value("negativeTags", tags(negative)),
    )
}

fn weighted(weight: f32, archetype: Option<u128>, results: Option<(i32, i32)>) -> TestInstance {
    let mut entry = instance("WeightedLootArchetype").value("weight", TestValue::Single(weight));
    if let Some(archetype) = archetype {
        entry = entry.value("archetype", TestValue::Reference(archetype));
    }
    if let Some((min, max)) = results {
        entry = entry.value(
            "numberOfResultsConstraints",
            TestValue::Struct(
                instance("NumResultsConstraints")
                    .value("minResults", TestValue::Int32(min))
                    .value("maxResults", TestValue::Int32(max)),
            ),
        );
    }
    entry
}

fn table(ids: &Ids) -> TestRecord {
    record(
        ids.table,
        "LootTable",
        "Cargo",
        &format!("{LOOT}loottables/cargo"),
    )
    .value(
        "lootArchetypes",
        array(vec![
            weighted(2.0, Some(ids.archetype), Some((0, 3))),
            weighted(1.0, None, None),
        ]),
    )
}

fn archetype(ids: &Ids) -> TestRecord {
    let stack = instance("EntryOptionalData_StackSize")
        .value("min", TestValue::Int32(3))
        .value("max", TestValue::Int32(5));
    let spawn = instance("EntryOptionalData_SpawnWith")
        .value("name", TestValue::text("Suitable ammunition"))
        .value("mode", TestValue::text("MostSimilar"))
        .value("min", TestValue::Int32(1))
        .value("max", TestValue::Int32(3))
        .value("tagsToMatch", term(&[ids.tag_common], &[ids.tag_rare]));
    let primary = instance("LootArchetypeEntry_Primary")
        .value("name", TestValue::text("Cargo"))
        .value("weight", TestValue::Single(1.0))
        .value("tag", TestValue::Reference(ids.tag_common))
        .value("additionalTags", term(&[ids.tag_rare], &[]))
        .value("optionalData", array(vec![stack, spawn]));
    let secondary = instance("LootArchetypeOrGroup_Secondary")
        .value("groupName", TestValue::text("Rarity"))
        .value(
            "entries",
            array(vec![
                instance("LootArchetypeEntry_Secondary")
                    .value("weight", TestValue::Single(4.0))
                    .value("tag", TestValue::Reference(ids.tag_rare)),
            ]),
        );
    record(
        ids.archetype,
        "LootArchetype",
        "LootArchetype_Cargo",
        &format!("{LOOT}lootarchetypes/lootarchetype_cargo"),
    )
    .value(
        "excludedTags",
        TestValue::Struct(instance("TagList").value("tags", tags(&[ids.tag_rare]))),
    )
    .value(
        "primaryOrGroup",
        TestValue::Struct(
            instance("LootArchetypeOrGroup_Primary").value("entries", array(vec![primary])),
        ),
    )
    .value("secondaryOrGroups", array(vec![secondary]))
}

/// Returns the older table and archetype Records.
pub(super) fn records(ids: &Ids) -> [TestRecord; 2] {
    [table(ids), archetype(ids)]
}
