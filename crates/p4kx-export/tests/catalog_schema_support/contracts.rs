// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! One contract with a pickup place and a reputation gain.

use serde_json::json;

pub(super) fn contracts() -> serde_json::Value {
    let places = json!([{
        "starmapObjectGuid": "a2000000-0000-4000-8000-000000000001",
        "objectType": "Planet",
        "place": {"key": "@Stanton1_Name", "en": "Hurston", "de": null}
    }]);
    let gains = json!([{
        "factionReputationGuid": "60000000-0000-4000-8000-000000000001",
        "factionReputationName": "acme",
        "scopeGuid": "60000000-0000-4000-8000-000000000002",
        "scopeName": "faction",
        "amount": 200
    }]);
    let mut entry = json!({
        "guid": "20000000-0000-4000-8000-000000000001",
        "subContractGuid": "30000000-0000-4000-8000-000000000002",
        "recordGuid": "10000000-0000-4000-8000-000000000001",
        "recordPath": "libs/foundry/records/contracts/contractgenerator/delivery",
        "debugName": "Deliver_A",
        "handler": "Delivery_Career",
        "templateGuid": "40000000-0000-4000-8000-000000000001",
        "notForRelease": false,
        "workInProgress": null,
        "title": {"key": "@title_a", "en": "Deliver Scrap to [LOCATION]", "de": null},
        "missionTypeGuid": "50000000-0000-4000-8000-000000000001",
        "missionType": {"key": "@type_delivery", "en": "Delivery", "de": null},
        "contractor": {"key": null, "en": null, "de": null},
        "faction": {
            "guid": "60000000-0000-4000-8000-000000000001",
            "name": "acme",
            "displayName": {"key": "@faction_acme", "en": "Acme Emporium", "de": null}
        },
        "pickupPlaces": places,
        "reputationGains": gains,
        "minStandingGuid": "62000000-0000-4000-8000-000000000001",
        "maxStandingGuid": null,
        "standingRequirements": [{
            "level": "contract",
            "exclude": false,
            "factionReputationGuid": "60000000-0000-4000-8000-000000000001",
            "scopeGuid": "61000000-0000-4000-8000-000000000001",
            "minStandingGuid": "62000000-0000-4000-8000-000000000001",
            "maxStandingGuid": null
        }],
        "cooldownLayer": "handler",
        "hasPersonalCooldown": true,
        "personalCooldownTime": 2880.0,
        "personalCooldownTimeVariation": 60.0,
        "abandonedCooldownTime": 30.0,
        "abandonedCooldownTimeVariation": null
    });
    if let (Some(target), serde_json::Value::Object(links)) = (entry.as_object_mut(), spawn_links())
    {
        target.extend(links);
    }
    json!([entry])
}

fn spawn_links() -> serde_json::Value {
    json!({
        "shipSpawns": [{
            "variable": "HostileShips_BP",
            "token": null,
            "level": "contract",
            "poolGuid": "e1000000-0000-4000-8000-000000000001"
        }],
        "locationQueries": [{
            "variable": "Location_BP",
            "token": "Location",
            "level": "template",
            "queryGuid": "e2000000-0000-4000-8000-000000000001"
        }],
        "locationPrerequisites": [{
            "level": "handler",
            "variable": "Location_BP",
            "token": null,
            "locationLevelType": "Planet"
        }]
    })
}
