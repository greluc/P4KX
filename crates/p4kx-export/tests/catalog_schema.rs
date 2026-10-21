//! The checked-in JSON Schema of the Basetool catalog matches the catalog types.
//!
//! Run with `P4KX_BLESS=1` to regenerate `schemas/basetool-catalog.schema.json`.

use std::error::Error;
use std::path::PathBuf;

use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

fn schema_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/basetool-catalog.schema.json")
}

fn generated() -> Result<String, Box<dyn Error>> {
    let mut text = serde_json::to_string_pretty(&p4kx_export::catalog_schema())?;
    text.push('\n');
    Ok(text)
}

#[test]
fn the_checked_in_catalog_schema_is_current() -> TestResult {
    let generated = generated()?;
    let path = schema_file();
    if std::env::var_os("P4KX_BLESS").is_some_and(|value| value == "1") {
        std::fs::write(&path, &generated)?;
    }
    let checked_in = std::fs::read_to_string(&path)?;
    assert!(
        checked_in == generated,
        "{} is stale; regenerate it with P4KX_BLESS=1 cargo test -p p4kx-export --test catalog_schema",
        path.display()
    );
    Ok(())
}

#[test]
fn the_catalog_schema_is_draft_2020_12() {
    let schema = p4kx_export::catalog_schema();
    assert_eq!(
        schema.get("$schema").and_then(serde_json::Value::as_str),
        Some("https://json-schema.org/draft/2020-12/schema"),
        "draft 2020-12"
    );
}

fn collect_properties<'s>(
    node: &'s serde_json::Value,
    found: &mut Vec<(&'s str, &'s serde_json::Value)>,
) {
    match node {
        serde_json::Value::Object(map) => {
            if let Some(properties) = map.get("properties").and_then(serde_json::Value::as_object) {
                found.extend(
                    properties
                        .iter()
                        .map(|(name, value)| (name.as_str(), value)),
                );
            }
            for value in map.values() {
                collect_properties(value, found);
            }
        }
        serde_json::Value::Array(items) => {
            for value in items {
                collect_properties(value, found);
            }
        }
        _ => {}
    }
}

#[test]
fn exactly_the_game_text_fields_carry_the_game_text_annotation() {
    let schema = p4kx_export::catalog_schema();
    let mut properties = Vec::new();
    collect_properties(&schema, &mut properties);
    let mut marked = 0;
    for (name, value) in properties {
        let carries = value.get("x-gameText") == Some(&json!(true));
        let game_text = name == "description" || name == "displayType";
        assert_eq!(carries, game_text, "x-gameText on {name}");
        marked += usize::from(carries);
    }
    assert_eq!(marked, 6, "four descriptions and two display types");
}

fn minimal_catalog() -> serde_json::Value {
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
            "blueprints": {"status": "complete"}
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
            ]
        }]
    })
}

#[test]
fn a_new_value_of_a_closed_value_set_is_additive() -> TestResult {
    let validator = jsonschema::validator_for(&p4kx_export::catalog_schema())?;
    let catalog = minimal_catalog();

    let mut unknown_class = catalog.clone();
    unknown_class["items"][0]["itemClass"] = json!("Ballistic");
    assert!(validator.is_valid(&unknown_class), "a new value is valid");

    let mut class_number = catalog.clone();
    class_number["items"][0]["itemClass"] = json!(3);
    assert!(
        !validator.is_valid(&class_number),
        "the item class stays a string"
    );

    let mut no_class = catalog.clone();
    no_class["items"][0]["itemClass"] = json!(null);
    assert!(validator.is_valid(&no_class), "the item class may be null");

    let mut unknown_playability = catalog.clone();
    unknown_playability["ships"][0]["playability"] = json!("event");
    assert!(
        validator.is_valid(&unknown_playability),
        "a new playability is valid"
    );

    let mut no_playability = catalog;
    no_playability["items"][0]["playability"] = json!(null);
    assert!(
        validator.is_valid(&no_playability),
        "the playability may be null"
    );
    Ok(())
}

#[test]
fn the_schema_accepts_unknown_fields_and_kinds_but_not_unknown_versions() -> TestResult {
    let validator = jsonschema::validator_for(&p4kx_export::catalog_schema())?;
    let catalog = minimal_catalog();
    assert!(validator.is_valid(&catalog), "a minimal catalog is valid");

    let mut version_two = catalog.clone();
    version_two["schemaVersion"] = json!(2);
    assert!(!validator.is_valid(&version_two), "only version 1");

    let mut upper_case = catalog.clone();
    upper_case["manufacturers"][0]["guid"] = json!("CFC0122D-5275-415A-A656-3DCB3346FEB5");
    assert!(!validator.is_valid(&upper_case), "lower-case GUIDs only");

    for kind in [
        "manufacturers",
        "items",
        "ships",
        "commodities",
        "blueprints",
    ] {
        let mut unknown = catalog.clone();
        unknown[kind][0]["extra"] = json!(true);
        assert!(validator.is_valid(&unknown), "unknown fields in {kind}");
    }

    let mut unknown_ingredient = catalog.clone();
    unknown_ingredient["blueprints"][0]["ingredients"][0]["extra"] = json!(true);
    assert!(
        validator.is_valid(&unknown_ingredient),
        "unknown fields in an ingredient"
    );

    let mut unknown_kind = catalog.clone();
    unknown_kind["loadouts"] = json!([{"guid": "x"}]);
    unknown_kind["completeness"]["loadouts"] = json!({"status": "complete"});
    unknown_kind["source"]["extra"] = json!(true);
    unknown_kind["completeness"]["items"]["extra"] = json!(true);
    unknown_kind["items"][0]["name"]["extra"] = json!(true);
    assert!(
        validator.is_valid(&unknown_kind),
        "unknown kinds and fields everywhere"
    );

    let mut no_quantity = catalog.clone();
    if let Some(ingredient) = no_quantity["blueprints"][0]["ingredients"][0].as_object_mut() {
        ingredient.remove("quantityScu");
    }
    assert!(
        !validator.is_valid(&no_quantity),
        "every field of an ingredient is present"
    );

    let mut missing = catalog;
    if let Some(ship) = missing["ships"][0].as_object_mut() {
        ship.remove("crewSize");
    }
    assert!(!validator.is_valid(&missing), "every field is present");
    Ok(())
}
