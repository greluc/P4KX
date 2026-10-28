// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The `crafters` kind of the minimal catalog.

use serde_json::json;

pub(super) fn crafters() -> serde_json::Value {
    json!([{
        "guid": "7000f0f0-0000-4000-8000-000000000001",
        "className": "basebuilding_interactables_itemfabricator_standard",
        "recordPath": "libs/foundry/records/entities/basebuilding/interactables/basebuilding_interactables_itemfabricator_standard",
        "maxJobsInHistory": 10,
        "queues": [{
            "name": "Fabricator",
            "maxJobsInProgress": 2,
            "maxJobsWaiting": 4
        }, {
            "name": null,
            "maxJobsInProgress": null,
            "maxJobsWaiting": null
        }]
    }, {
        "guid": "7000f0f0-0000-4000-8000-000000000002",
        "className": "bare_crafter",
        "recordPath": "libs/foundry/records/entities/basebuilding/bare_crafter",
        "maxJobsInHistory": null,
        "queues": []
    }])
}
