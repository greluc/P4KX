//! A minimal catalog that holds one entity of every kind and every field the schema lists.

use serde_json::json;

pub(crate) fn minimal_catalog() -> serde_json::Value {
    json!({
        "schemaVersion": 1,
        "source": {
            "channel": null,
            "build": null,
            "archiveSize": 0,
            "archiveModified": null,
            "toolVersion": "0.1.0",
            "exportedAt": "2026-10-18T00:00:00Z"
        },
        "completeness": {
            "manufacturers": {"status": "complete"},
            "items": {"status": "complete"},
            "ships": {"status": "partial", "filter": "*/spaceships/*"},
            "commodities": {"status": "complete"},
            "blueprints": {"status": "complete"},
            "resourceGroups": {"status": "complete"},
            "blueprintCategories": {"status": "partial", "filter": "*/blueprintcategories/*"},
            "gameplayProperties": {"status": "complete"},
            "qualityDistributions": {"status": "complete"},
            "blueprintPools": {"status": "complete"},
            "dismantleProcesses": {"status": "complete"},
            "paints": {"status": "complete"}
        },
        "manufacturers": [{
            "guid": "cfc0122d-5275-415a-a656-3dcb3346feb5",
            "className": "ACAS",
            "recordPath": "libs/foundry/records/scitemmanufacturer/acas",
            "code": "ACAS",
            "name": {"key": "@manufacturer_NameACAS", "en": "Ace Astrogation", "de": null},
            "description": {"key": null, "en": null, "de": null}
        }],
        "items": [{
            "guid": "30000000-0000-4000-8000-000000000001",
            "className": "POWR_ACAS_S01",
            "recordPath": "libs/foundry/records/entities/scitem/ships/powerplant/powr_acas_s01",
            "name": {"key": "@item_NamePOWR_ACAS_S01", "en": "Lumen", "de": null},
            "description": {"key": null, "en": null, "de": null},
            "type": "PowerPlant",
            "subType": "Power",
            "size": 1,
            "grade": 3,
            "mass": 125.5,
            "manufacturerGuid": "cfc0122d-5275-415a-a656-3dcb3346feb5",
            "tags": ["flightReady"],
            "displayType": {"key": null, "en": null, "de": null},
            "itemClass": "Military",
            "playability": "player"
        }],
        "ships": [{
            "guid": "90000000-0000-4000-8000-000000000001",
            "className": "ACAS_Ship",
            "recordPath": "libs/foundry/records/entities/spaceships/acas_ship",
            "name": {"key": null, "en": null, "de": null},
            "description": {"key": null, "en": null, "de": null},
            "role": {"key": "@vehicle_class_lightfreight", "en": "Light Freight", "de": null},
            "career": {"key": null, "en": null, "de": null},
            "crewSize": 2,
            "manufacturerGuid": null,
            "vehicleDefinition": null,
            "vehicleKind": "spaceship",
            "size": 3,
            "mass": 1210.0,
            "massBasis": "parts",
            "hullHealth": 600.0,
            "cargoScu": 18.5,
            "cargoGridsWithIssue": 0,
            "playability": "ai"
        }],
        "commodities": [{
            "guid": "a1000000-0000-4000-8000-000000000001",
            "className": "gold",
            "recordPath": "libs/foundry/records/entities/commodities/metals/gold",
            "name": {"key": "@items_commodities_gold", "en": "Gold", "de": null},
            "description": {"key": null, "en": null, "de": null},
            "typeGuid": "b1000000-0000-4000-8000-000000000001",
            "subtypeGuid": null,
            "displayType": {"key": null, "en": null, "de": null}
        }],
        "blueprints": [{
            "guid": "11000000-0000-4000-8000-000000000001",
            "className": "BP_CRAFT_core",
            "recordPath": "libs/foundry/records/crafting/blueprints/crafting/bp_craft_core",
            "key": "BP_CRAFT_core",
            "name": {"key": null, "en": null, "de": null},
            "categoryGuid": "d1000000-0000-4000-8000-000000000001",
            "producedItemGuid": null,
            "craftTimeSeconds": 93784,
            "ingredients": [
                {
                    "resourceGuid": "b1000000-0000-4000-8000-000000000001",
                    "itemGuid": null,
                    "quantityScu": 0.5,
                    "quantityUnits": null,
                    "minQuality": 0,
                    "slot": "CORE",
                    "slotKey": null
                },
                {
                    "resourceGuid": null,
                    "itemGuid": "30000000-0000-4000-8000-000000000001",
                    "quantityScu": null,
                    "quantityUnits": 5,
                    "minQuality": 500,
                    "slot": null,
                    "slotKey": "@crafting_ui_slotname_core"
                }
            ],
            "tiers": tiers(),
            "defaultAvailable": false,
            "dismantle": {
                "processGuid": "f1000000-0000-4000-8000-000000000001",
                "tierIndex": 0
            }
        }],
        "resourceGroups": [{
            "guid": "b1000000-0000-4000-8000-000000000001",
            "className": "Metal",
            "recordPath": "libs/foundry/records/resourcetypedatabase/resourcetypedatabase",
            "name": {"key": "@items_commodities_type_metal", "en": "Metal", "de": null},
            "parentGuid": null
        }],
        "blueprintCategories": [{
            "guid": "d1000000-0000-4000-8000-000000000001",
            "className": "Ammo",
            "recordPath": "libs/foundry/records/crafting/blueprintcategories/blueprintcategorydatabase",
            "name": {"key": null, "en": null, "de": null}
        }],
        "gameplayProperties": [{
            "guid": "e1000000-0000-4000-8000-000000000001",
            "className": "GPP_Armor",
            "recordPath": "libs/foundry/records/crafting/craftedproperties/gpp_armor",
            "name": {"key": "@StatName_GPP_Armor", "en": "Armour", "de": null}
        }],
        "qualityDistributions": [{
            "guid": "e2000000-0000-4000-8000-000000000001",
            "className": "Creature_Pyro",
            "recordPath": "libs/foundry/records/crafting/qualitydistribution/creature_pyro",
            "distribution": null,
            "overrides": [{
                "locationGuid": "e3000000-0000-4000-8000-000000000001",
                "distribution": {
                    "kind": "CraftingQualityDistributionNormal",
                    "min": 1.0, "max": 1000.0, "mean": 269.0, "stddev": null
                }
            }]
        }],
        "blueprintPools": pools(),
        "dismantleProcesses": [{
            "guid": "f1000000-0000-4000-8000-000000000001",
            "className": "GlobalGenericDismantle",
            "recordPath": "libs/foundry/records/crafting/blueprints/dismantle/globalgenericdismantle",
            "efficiency": 0.5,
            "timeSeconds": 15
        }],
        "craftingGlobal": {
            "dismantleExcludedResourceGuids": ["b1000000-0000-4000-8000-000000000001"],
            "dismantleExcludedItemGuids": []
        },
        "paints": [{
            "itemGuid": "30000000-0000-4000-8000-000000000001",
            "shipGuids": ["90000000-0000-4000-8000-000000000001"]
        }]
    })
}

fn pools() -> serde_json::Value {
    json!([{
        "guid": "e4000000-0000-4000-8000-000000000001",
        "className": "BP_REWARDS_Haul",
        "recordPath": "libs/foundry/records/crafting/blueprintrewards/bp_rewards_haul",
        "totalWeight": 4.0,
        "entries": [{
            "blueprintGuid": "11000000-0000-4000-8000-000000000001",
            "weight": 1.0
        }],
        "rewardSources": [{
            "recordGuid": "e5000000-0000-4000-8000-000000000001",
            "recordPath": "libs/foundry/records/contracts/contractgenerator/haul",
            "contractId": null, "contractName": "Haul_Intro", "chance": 0.5
        }]
    }])
}

fn tiers() -> serde_json::Value {
    let line = json!({
        "kind": "resource", "count": null, "slot": null, "slotKey": null,
        "options": [],
        "resourceGuid": "b1000000-0000-4000-8000-000000000001",
        "itemGuid": null, "quantityScu": 0.5, "quantityUnits": null,
        "minQuality": 0, "quantityMultiplier": 1.15,
        "compositionInclusion": "Include",
        "modifiers": [{
            "propertyGuid": "e1000000-0000-4000-8000-000000000001",
            "ranges": [{
                "kind": "multiplier", "startQuality": 0, "endQuality": 1000,
                "atStart": 0.85, "atEnd": 1.15
            }]
        }]
    });
    json!([{
        "craftTimeSeconds": 93784,
        "mandatory": {
            "kind": "select", "count": 1, "slot": "CORE", "slotKey": null,
            "options": [line],
            "resourceGuid": null, "itemGuid": null, "quantityScu": null,
            "quantityUnits": null, "minQuality": null, "quantityMultiplier": null,
            "compositionInclusion": null, "modifiers": []
        },
        "optional": []
    }])
}
