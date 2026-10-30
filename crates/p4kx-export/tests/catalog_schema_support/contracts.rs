// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! One contract with a pickup place, a reputation gain and a hauling order, and a cargo manifest.

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
    json!([{
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
        "haulingOrders": orders()
    }])
}

fn orders() -> serde_json::Value {
    json!([{
        "source": "contract",
        "variableName": "HaulingOverride",
        "requirements": [{"options": [[{
            "kind": "resource",
            "resourceGuid": "f5000000-0000-4000-8000-000000000001",
            "entityClassGuid": null,
            "entityClassesGuid": null,
            "minScu": 4,
            "maxScu": 8,
            "maxContainerSize": 16,
            "minAmount": null,
            "maxAmount": null
        }]]}]
    }])
}

pub(super) fn manifests() -> serde_json::Value {
    json!([{
        "guid": "f7000000-0000-4000-8000-000000000001",
        "className": "LegalCargo_Even",
        "recordPath": "libs/foundry/records/cargomanifest/civilian/legalcargo_even",
        "fillType": "even",
        "minCapacityRange": 0.45,
        "maxCapacityRange": 0.5,
        "lockCargoGridsOnSpawn": false,
        "resources": [{
            "resourceGuid": "f5000000-0000-4000-8000-000000000001",
            "probability": 1.0,
            "minAllocation": null,
            "maxAllocation": null
        }],
        "spawnRules": [{
            "kind": "entities",
            "probability": 1.0,
            "entityGuids": ["f6000000-0000-4000-8000-000000000001"],
            "resourceGuids": [],
            "quantityAlgorithm": "CargoManifestFixedQuantityDef",
            "quantity": 30,
            "proportion": null
        }]
    }])
}
