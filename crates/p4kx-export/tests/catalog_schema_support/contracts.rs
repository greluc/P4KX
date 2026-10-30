// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! One contract with a pickup place, a reputation gain and the contract facts.

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
        "abandonedCooldownTimeVariation": null,
        "fixedRewards": [{"currency": "UEC", "amount": 40000, "max": 0, "plusBonuses": false}]
    });
    if let (Some(entry), Some(facts)) = (entry.as_object_mut(), facts().as_object()) {
        entry.extend(facts.clone());
    }
    json!([entry])
}

fn facts() -> serde_json::Value {
    json!({
        "timeToComplete": 20.0,
        "buyInAmount": 0,
        "canBeShared": {"value": true, "layer": "template"},
        "illegal": {"value": false, "layer": "contract"},
        "onceOnly": {"value": true, "layer": "subContract"},
        "maxPlayersPerInstance": {"value": 1, "layer": "handler"},
        "availableInPrison": null,
        "canReacceptAfterAbandoning": {"value": true, "layer": "handler"},
        "canReacceptAfterFailing": null,
        "hideInMobiGlas": null,
        "requiredIntros": [{
            "level": "handler",
            "tagGuid": "90000000-0000-4000-8000-000000000002",
            "introContracts": ["20000000-0000-4000-8000-000000000001"]
        }]
    })
}
