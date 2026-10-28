// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! One entity of each faction, reputation and mission kind.

use serde_json::json;

const SCOPE: &str = "61000000-0000-4000-8000-000000000001";
const STANDING: &str = "62000000-0000-4000-8000-000000000001";

pub(super) fn factions() -> serde_json::Value {
    json!([{
        "guid": "60000000-0000-4000-8000-000000000001",
        "className": "FactionReputation_Acme",
        "recordPath": "libs/foundry/records/factions/factionreputation/factionreputation_acme",
        "displayName": {"key": "@faction_acme", "en": "Acme Emporium", "de": null},
        "isNpc": false,
        "lawful": true,
        "contextGuid": "63000000-0000-4000-8000-000000000001",
        "allyGuids": [],
        "enemyGuids": ["60000000-0000-4000-8000-000000000002"],
        "hostility": {"scopeGuid": SCOPE, "standingGuid": STANDING},
        "allied": null
    }])
}

pub(super) fn scopes() -> serde_json::Value {
    json!([{
        "guid": SCOPE,
        "className": "FactionReputationScope",
        "recordPath": "libs/foundry/records/reputation/scopes/factionreputationscope",
        "scopeName": "FactionReputation",
        "displayName": {"key": "@scope_standing", "en": "Standing", "de": null},
        "reputationCeiling": 95250,
        "initialReputation": 0,
        "standingGuids": [STANDING]
    }])
}

pub(super) fn standings() -> serde_json::Value {
    json!([{
        "guid": STANDING,
        "className": "ReputationStanding_Faction_B0_00",
        "recordPath": "libs/foundry/records/reputation/standings/faction/band_0/reputationstanding_faction_b0_00",
        "displayName": {"key": null, "en": null, "de": null},
        "minReputation": 0,
        "driftReputation": -5,
        "driftTimeHours": 24,
        "gated": false
    }])
}

pub(super) fn contexts() -> serde_json::Value {
    json!([{
        "guid": "63000000-0000-4000-8000-000000000001",
        "className": "ReputationContext_Transport",
        "recordPath": "libs/foundry/records/reputation/contexts/reputationcontext_transport",
        "primaryScopeGuid": SCOPE,
        "scopeGuids": [SCOPE]
    }])
}

pub(super) fn mission_types() -> serde_json::Value {
    json!([{
        "guid": "50000000-0000-4000-8000-000000000001",
        "className": "missiontype.job",
        "recordPath": "libs/foundry/records/missiontype/missiontype.job",
        "typeName": {"key": "@type_job", "en": "Job", "de": null}
    }])
}

pub(super) fn localities() -> serde_json::Value {
    json!([{
        "guid": "64000000-0000-4000-8000-000000000001",
        "className": "pyro",
        "recordPath": "libs/foundry/records/missiondata/pu_missionlocality/pyro",
        "starmapObjectGuids": ["a2000000-0000-4000-8000-000000000001"]
    }])
}

pub(super) fn scenarios() -> serde_json::Value {
    json!([{
        "guid": "65000000-0000-4000-8000-000000000001",
        "className": "ORS_ScenarioProgress",
        "recordPath": "libs/foundry/records/contracts/contractscenarios/ors_scenarioprogress",
        "factionRewardTiers": [{
            "factionGuid": "66000000-0000-4000-8000-000000000001",
            "factionName": "faction_lawful_crusaderindustries",
            "pointsAsPercent": false,
            "progressions": [{
                "minMultiplierThreshold": 1.0,
                "completionType": "CompletionType_GlobalProgression",
                "periodScenarioGuid": null,
                "tiers": [{
                    "minPoints": 10800,
                    "badgeToAward": "ORS_Pistol",
                    "globalPointsMultiplier": 1.0,
                    "blueprintPoolGuids": ["e4000000-0000-4000-8000-000000000001"]
                }]
            }]
        }]
    }])
}
