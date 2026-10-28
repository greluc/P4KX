// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! One entity of each loot kind, with every field the schema lists.

use serde_json::json;

fn selector() -> serde_json::Value {
    json!({
        "selectorType": "LootArchetypeV3Selector_Tags",
        "positiveTagGuids": ["a9000000-0000-4000-8000-000000000010"],
        "negativeTagGuids": ["a9000000-0000-4000-8000-000000000011"],
        "entityClassGuids": ["a9000000-0000-4000-8000-000000000012"]
    })
}

fn archetype_entry() -> serde_json::Value {
    json!({
        "name": "Ammo",
        "weight": 3.0,
        "selector": selector(),
        "stackSize": {"min": 1.0, "max": 3.0},
        "spawnWith": [{
            "name": "Suitable ammo",
            "chance": 1.0,
            "mode": "MostSimilar",
            "amount": {"min": 2.0, "max": 4.0},
            "selector": selector()
        }]
    })
}

pub(super) fn tables() -> serde_json::Value {
    json!([{
        "guid": "a9000000-0000-4000-8000-000000000001",
        "className": "V3LootTable_Actor_CQC",
        "recordPath": "libs/foundry/records/lootgeneration/loottables/v3loottable_actor_cqc",
        "entries": [{
            "name": "Ammo",
            "weight": 10.0,
            "archetypeGuid": null,
            "archetype": [archetype_entry()],
            "priorityCount": 1,
            "choiceLimit": 2,
            "dupeLimit": 3,
            "chanceToExist": 0.75
        }, {
            "name": "Medpens",
            "weight": 1.0,
            "archetypeGuid": "a9000000-0000-4000-8000-000000000002",
            "archetype": null,
            "priorityCount": null,
            "choiceLimit": null,
            "dupeLimit": null,
            "chanceToExist": null
        }]
    }])
}

pub(super) fn archetypes() -> serde_json::Value {
    json!([{
        "guid": "a9000000-0000-4000-8000-000000000002",
        "className": "V3LootArchetype_Medpen_Healing",
        "recordPath": "libs/foundry/records/lootgeneration/lootarchetypes/v3lootarchetype_medpen_healing",
        "entries": [archetype_entry()]
    }])
}

pub(super) fn filters() -> serde_json::Value {
    json!([{
        "guid": "a9000000-0000-4000-8000-000000000003",
        "className": "V3PoolFilter_ASDDelving",
        "recordPath": "libs/foundry/records/lootgeneration/filters/v3poolfilter_asddelving",
        "filter": {
            "nodeType": "PoolFilter_Sequence",
            "filterGuid": null,
            "positiveTagGuids": [],
            "negativeTagGuids": [],
            "filters": [{
                "name": "No faction",
                "mode": "Cumulative",
                "filter": {
                    "nodeType": "PoolFilter_RecordRef",
                    "filterGuid": "a9000000-0000-4000-8000-000000000004",
                    "positiveTagGuids": [],
                    "negativeTagGuids": [],
                    "filters": []
                }
            }, {
                "name": "Event",
                "mode": "Additive",
                "filter": null
            }]
        }
    }])
}

pub(super) fn containers() -> serde_json::Value {
    json!([{
        "multiConfigGuid": "a7000000-0000-4000-8000-000000000001",
        "container": "Data\\ObjectContainers\\PU\\loc\\mod\\drlct\\loot_area.socpak",
        "system": "stanton",
        "starmapObjectGuid": "a2000000-0000-4000-8000-000000000001",
        "entityCount": 13,
        "tagCounts": [{"tagGuid": "a7000000-0000-4000-8000-000000000002", "entities": 4}]
    }, {
        "multiConfigGuid": "a7000000-0000-4000-8000-000000000001",
        "container": "ObjectContainers/PU/loc/missing.socpak",
        "system": "pyro",
        "starmapObjectGuid": null,
        "entityCount": null,
        "tagCounts": []
    }])
}
