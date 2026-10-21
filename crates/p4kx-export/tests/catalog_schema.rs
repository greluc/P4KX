//! The checked-in JSON Schema of the Basetool catalog matches the catalog types.
//!
//! Run with `P4KX_BLESS=1` to regenerate `schemas/basetool-catalog.schema.json`.

mod catalog_schema_support;

use std::error::Error;
use std::path::PathBuf;

use catalog_schema_support::minimal_catalog;
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
        "resourceGroups",
        "blueprintCategories",
        "gameplayProperties",
        "qualityDistributions",
        "blueprintPools",
        "dismantleProcesses",
    ] {
        let mut unknown = catalog.clone();
        unknown[kind][0]["extra"] = json!(true);
        assert!(validator.is_valid(&unknown), "unknown fields in {kind}");
    }

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

    let mut unknown_ingredient = catalog;
    unknown_ingredient["blueprints"][0]["ingredients"][0]["extra"] = json!(true);
    assert!(
        validator.is_valid(&unknown_ingredient),
        "unknown fields in an ingredient"
    );
    Ok(())
}

#[test]
fn every_field_the_schema_lists_is_required() -> TestResult {
    let validator = jsonschema::validator_for(&p4kx_export::catalog_schema())?;
    for (kind, field, what) in [
        ("blueprints", "tiers", "a blueprint lists its tiers"),
        (
            "blueprints",
            "dismantle",
            "a blueprint states its dismantle",
        ),
        ("resourceGroups", "parentGuid", "a resource group"),
        ("ships", "crewSize", "a ship"),
        ("blueprintPools", "rewardSources", "a pool"),
    ] {
        let mut catalog = minimal_catalog();
        if let Some(entity) = catalog[kind][0].as_object_mut() {
            entity.remove(field);
        }
        assert!(!validator.is_valid(&catalog), "{what} has {field}");
    }
    let mut no_quantity = minimal_catalog();
    if let Some(ingredient) = no_quantity["blueprints"][0]["ingredients"][0].as_object_mut() {
        ingredient.remove("quantityScu");
    }
    assert!(
        !validator.is_valid(&no_quantity),
        "every field of an ingredient is present"
    );
    let mut no_modifiers = minimal_catalog();
    if let Some(line) = no_modifiers["blueprints"][0]["tiers"][0]["mandatory"].as_object_mut() {
        line.remove("modifiers");
    }
    assert!(
        !validator.is_valid(&no_modifiers),
        "every field of a cost line is present"
    );
    let mut no_global = minimal_catalog();
    if let Some(catalog) = no_global.as_object_mut() {
        catalog.remove("craftingGlobal");
    }
    assert!(!validator.is_valid(&no_global), "the global parameters");
    Ok(())
}

#[test]
fn the_crafting_fields_accept_additions_and_new_closed_values_but_stay_required() -> TestResult {
    let validator = jsonschema::validator_for(&p4kx_export::catalog_schema())?;
    let catalog = minimal_catalog();
    let mut unknown_line = catalog.clone();
    unknown_line["blueprints"][0]["tiers"][0]["mandatory"]["options"][0]["extra"] = json!(true);
    unknown_line["blueprints"][0]["dismantle"]["extra"] = json!(true);
    unknown_line["craftingGlobal"]["extra"] = json!(true);
    assert!(
        validator.is_valid(&unknown_line),
        "unknown fields in a cost line, a dismantle link and the global parameters"
    );

    let mut new_kind_value = catalog;
    new_kind_value["blueprints"][0]["tiers"][0]["mandatory"]["kind"] = json!("bundle");
    new_kind_value["blueprints"][0]["tiers"][0]["mandatory"]["options"][0]["modifiers"][0]["ranges"]
        [0]["kind"] = json!("curve");
    assert!(
        validator.is_valid(&new_kind_value),
        "a new line or range kind is additive: the closed sets stay strings"
    );

    Ok(())
}
