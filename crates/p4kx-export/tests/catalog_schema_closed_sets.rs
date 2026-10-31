// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The closed value sets of the catalog schema are strings that list their known values (ADR-0054).

#![recursion_limit = "256"]

mod catalog_schema_support;

use catalog_schema_support::minimal_catalog;
use serde_json::{Value, json};

/// Collects the JSON pointer of every `enum` and every `oneOf` / `anyOf` whose variants all carry
/// a `const`.
fn closed_set_violations(node: &Value, pointer: &str, found: &mut Vec<String>) {
    match node {
        Value::Object(object) => {
            if object.contains_key("enum") {
                found.push(format!("{pointer}/enum"));
            }
            for keyword in ["oneOf", "anyOf"] {
                if let Some(Value::Array(variants)) = object.get(keyword)
                    && !variants.is_empty()
                    && variants
                        .iter()
                        .all(|variant| variant.get("const").is_some())
                {
                    found.push(format!("{pointer}/{keyword}"));
                }
            }
            for (key, child) in object {
                closed_set_violations(child, &format!("{pointer}/{key}"), found);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                closed_set_violations(child, &format!("{pointer}/{index}"), found);
            }
        }
        _ => {}
    }
}

#[test]
fn no_schema_node_is_an_enum_or_a_oneof_of_constants() {
    let mut found = Vec::new();
    closed_set_violations(&p4kx_export::catalog_schema(), "", &mut found);
    assert!(
        found.is_empty(),
        "a closed value set stays a string with x-values (ADR-0054): {found:?}"
    );
}

#[test]
fn the_rule_test_finds_a_oneof_of_constants_and_an_enum() {
    let mut found = Vec::new();
    let schema = json!({
        "$defs": {
            "A": {"oneOf": [{"const": "x"}, {"const": "y"}]},
            "B": {"properties": {"p": {"enum": ["x"]}}},
            "C": {"anyOf": [{"const": "x"}]},
            "D": {"oneOf": [{"const": "x"}, {"type": "object"}]}
        }
    });
    closed_set_violations(&schema, "", &mut found);
    found.sort();
    assert_eq!(
        found,
        [
            "/$defs/A/oneOf",
            "/$defs/B/properties/p/enum",
            "/$defs/C/anyOf"
        ]
    );
}

#[test]
fn the_derived_closed_sets_are_strings_that_list_their_known_values() {
    let schema = p4kx_export::catalog_schema();
    for name in [
        "FactLayer",
        "StandingLevel",
        "SpawnLevel",
        "HaulingSource",
        "HaulingItemKind",
        "LocationCount",
        "CargoFillType",
        "CargoSpawnKind",
    ] {
        let definition = &schema["$defs"][name];
        assert_eq!(definition["type"], "string", "{name} is a string");
        assert!(
            definition["x-values"]
                .as_array()
                .is_some_and(|values| !values.is_empty()),
            "{name} lists its known values"
        );
    }
}

#[test]
fn a_new_value_of_a_derived_closed_set_is_additive() -> Result<(), Box<dyn std::error::Error>> {
    let validator = jsonschema::validator_for(&p4kx_export::catalog_schema())?;
    let catalog = minimal_catalog();
    assert!(validator.is_valid(&catalog), "the minimal catalog is valid");

    let mut new_layer = catalog.clone();
    new_layer["contracts"][0]["canBeShared"]["layer"] = json!("newLayer");
    new_layer["contracts"][0]["cooldownLayer"] = json!("newLayer");
    assert!(validator.is_valid(&new_layer), "a new fact layer is valid");

    let mut new_level = catalog.clone();
    new_level["contracts"][0]["standingRequirements"][0]["level"] = json!("newLevel");
    new_level["contracts"][0]["requiredIntros"][0]["level"] = json!("newLevel");
    new_level["contracts"][0]["shipSpawns"][0]["level"] = json!("newLevel");
    assert!(
        validator.is_valid(&new_level),
        "a new standing or spawn level is valid"
    );

    let mut layer_number = catalog;
    layer_number["contracts"][0]["canBeShared"]["layer"] = json!(3);
    assert!(
        !validator.is_valid(&layer_number),
        "a fact layer stays a string"
    );
    Ok(())
}
