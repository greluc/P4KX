// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Schema transform for the closed value sets of the catalog.

use schemars::Schema;
use serde_json::{Map, Value, json};

/// Rewrites the `oneOf` of string constants schemars derives for a unit-variant enum into a plain
/// string schema.
///
/// A closed value set the game may extend stays a string, so a catalog with a new value still
/// validates; the known values are listed in `x-values` as `{value, description}` entries.
pub(crate) fn closed_string_set(schema: &mut Schema) {
    let Some(object) = schema.as_object_mut() else {
        return;
    };
    let Some(Value::Array(variants)) = object.remove("oneOf") else {
        return;
    };
    let values: Vec<Value> = variants.iter().map(known_value).collect();
    object.insert("type".to_owned(), json!("string"));
    object.insert("x-values".to_owned(), Value::Array(values));
}

/// The `{value, description}` entry of one derived variant.
fn known_value(variant: &Value) -> Value {
    let mut entry = Map::new();
    if let Some(value) = variant.get("const") {
        entry.insert("value".to_owned(), value.clone());
    }
    if let Some(description) = variant.get("description") {
        entry.insert("description".to_owned(), description.clone());
    }
    Value::Object(entry)
}
