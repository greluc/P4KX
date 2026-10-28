// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The `crafters` kind of the minimal catalog.

use serde_json::json;

pub(super) fn crafters() -> serde_json::Value {
    json!([{
        "guid": "7000f0f0-0000-4000-8000-000000000001",
        "className": "basebuilding_interactables_itemfabricator_standard",
        "recordPath": "libs/foundry/records/entities/basebuilding/interactables/basebuilding_interactables_itemfabricator_standard",
        "type": "Crafter",
        "subType": "UNDEFINED",
        "size": 1,
        "maxJobsInHistory": 10,
        "baseCraftingTime": 1.0,
        "queues": [{
            "name": "Fabricator",
            "maxJobsInProgress": 2,
            "maxJobsWaiting": 4,
            "blueprintCategoryAvailability": {
                "kind": "whitelist",
                "structName": "BlueprintCategoryAvailability_Whitelist",
                "blueprintCategoryGuids": ["7000f0f0-0000-4000-8000-0000000000c1"]
            },
            "processTypeAvailability": [true, false]
        }, {
            "name": null,
            "maxJobsInProgress": null,
            "maxJobsWaiting": null,
            "blueprintCategoryAvailability": {
                "kind": "none",
                "structName": null,
                "blueprintCategoryGuids": []
            },
            "processTypeAvailability": []
        }],
        "modifiers": [{
            "type": "time",
            "structName": "CraftingModifierType_Time",
            "timeMultiplier": 0.7,
            "condition": {
                "kind": "blueprintCategory",
                "structName": "CraftingModifierCondition_BlueprintCategory",
                "blueprintCategoryGuids": ["7000f0f0-0000-4000-8000-0000000000c1"]
            }
        }],
        "issues": [],
        "hostGuids": ["7000f0f0-0000-4000-8000-0000000000aa"]
    }, {
        "guid": "7000f0f0-0000-4000-8000-000000000002",
        "className": "bare_crafter",
        "recordPath": "libs/foundry/records/entities/basebuilding/bare_crafter",
        "type": null,
        "subType": null,
        "size": null,
        "maxJobsInHistory": null,
        "baseCraftingTime": null,
        "queues": [],
        "modifiers": [],
        "issues": ["unknown_modifier_type"],
        "hostGuids": []
    }])
}
