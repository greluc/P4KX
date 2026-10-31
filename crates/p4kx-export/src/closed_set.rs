// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Schema transform for the closed value sets of the catalog.

use schemars::Schema;
use serde_json::{Map, Value, json};

/// Rewrites the string constants schemars derives for a unit-variant enum into a plain string
/// schema.
///
/// schemars writes a `oneOf` of `const` variants when a variant has a doc comment, else an `enum`
/// array. A closed value set the game may extend stays a string, so a catalog with a new value
/// still validates; the known values are listed in `x-values` as `{value, description}` entries
/// (`description` only where the variant has one). A schema of any other shape stays untouched.
pub(crate) fn closed_string_set(schema: &mut Schema) {
    let Some(object) = schema.as_object_mut() else {
        return;
    };
    let Some(values) = known_values(object) else {
        return;
    };
    object.remove("oneOf");
    object.remove("enum");
    object.insert("type".to_owned(), json!("string"));
    object.insert("x-values".to_owned(), Value::Array(values));
}

/// The `x-values` entries of a schema made only of string constants, `None` for any other shape.
fn known_values(object: &Map<String, Value>) -> Option<Vec<Value>> {
    if let Some(Value::Array(variants)) = object.get("oneOf") {
        return variants.iter().map(variant_entry).collect();
    }
    let Some(Value::Array(values)) = object.get("enum") else {
        return None;
    };
    values
        .iter()
        .map(|value| value.is_string().then(|| json!({ "value": value })))
        .collect()
}

/// The entry of one `oneOf` variant, `None` unless it carries a string `const`.
fn variant_entry(variant: &Value) -> Option<Value> {
    let value = variant.get("const").filter(|value| value.is_string())?;
    let mut entry = Map::new();
    entry.insert("value".to_owned(), value.clone());
    if let Some(description) = variant.get("description") {
        entry.insert("description".to_owned(), description.clone());
    }
    Some(Value::Object(entry))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transformed(schema: Value) -> Value {
        let mut schema = Schema::try_from(schema).expect("an object schema");
        closed_string_set(&mut schema);
        schema.to_value()
    }

    #[test]
    fn a_oneof_of_documented_constants_becomes_a_string_with_its_values() {
        let schema = transformed(json!({
            "description": "Set.",
            "oneOf": [
                {"const": "a", "description": "The a.", "type": "string"},
                {"const": "b", "type": "string"}
            ]
        }));
        assert_eq!(
            schema,
            json!({
                "description": "Set.",
                "type": "string",
                "x-values": [{"value": "a", "description": "The a."}, {"value": "b"}]
            })
        );
    }

    #[test]
    fn an_enum_array_becomes_a_string_with_its_values() {
        let schema = transformed(json!({"type": "string", "enum": ["a", "b"]}));
        assert_eq!(
            schema,
            json!({"type": "string", "x-values": [{"value": "a"}, {"value": "b"}]})
        );
    }

    #[test]
    fn a_mixed_oneof_stays_untouched() {
        let mixed = json!({
            "oneOf": [
                {"const": "a", "type": "string"},
                {"type": "object", "properties": {}}
            ]
        });
        assert_eq!(transformed(mixed.clone()), mixed);
    }

    #[test]
    fn a_non_string_constant_stays_untouched() {
        let numbers = json!({"oneOf": [{"const": 1}, {"const": 2}]});
        assert_eq!(transformed(numbers.clone()), numbers);
        let enum_numbers = json!({"enum": [1, 2]});
        assert_eq!(transformed(enum_numbers.clone()), enum_numbers);
    }
}
