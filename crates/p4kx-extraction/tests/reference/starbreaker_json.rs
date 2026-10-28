// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Reads the `DataForge` JSON of `StarBreaker` into the value model of the comparison.
//!
//! The key names below are the one place that encodes the output shape of `StarBreaker` (CLI 0.3.2,
//! `dcb extract --format json`): a Record file is an object with the Record name, GUID, tag and
//! root struct; a struct carries its type and, when a pointer targets it, a pointer id; a
//! Reference is an object with a Record GUID (with a Record path, or with the content of a
//! Record of the same file embedded), or a bare `file://` path; a weak pointer is a string that
//! points to a pointer id.

use serde_json::Value;

use super::value_tree::{JsonNumber, Node, RecordDoc, sorted};

pub(crate) const RECORD_NAME_KEY: &str = "_RecordName_";
pub(crate) const RECORD_ID_KEY: &str = "_RecordId_";
pub(crate) const RECORD_PATH_KEY: &str = "_RecordPath_";
pub(crate) const RECORD_TAG_KEY: &str = "_RecordTag_";
pub(crate) const RECORD_VALUE_KEY: &str = "_RecordValue_";
pub(crate) const TYPE_KEY: &str = "_Type_";
pub(crate) const POINTER_KEY: &str = "_Pointer_";
pub(crate) const POINTS_TO_PREFIX: &str = "_PointsTo_:";
pub(crate) const FILE_PREFIX: &str = "file://";

const LINK_KEYS: [&str; 3] = [RECORD_ID_KEY, RECORD_PATH_KEY, RECORD_NAME_KEY];

fn number(value: &serde_json::Number) -> JsonNumber {
    value.as_i64().map_or_else(
        || {
            value.as_u64().map_or_else(
                || JsonNumber::Float(value.as_f64().unwrap_or(f64::NAN)),
                JsonNumber::Unsigned,
            )
        },
        JsonNumber::Signed,
    )
}

fn text_of(map: &serde_json::Map<String, Value>, key: &str) -> String {
    map.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn string(text: &str) -> Node {
    if text.starts_with(POINTS_TO_PREFIX) {
        Node::WeakPointer
    } else if text.starts_with(FILE_PREFIX) {
        Node::Reference {
            guid: String::new(),
            path: text.to_owned(),
            embedded: false,
        }
    } else {
        Node::Text(text.to_owned())
    }
}

/// Converts one JSON value of a Record.
#[must_use]
pub(crate) fn node(value: &Value) -> Node {
    match value {
        Value::Null => Node::Null,
        Value::Bool(flag) => Node::Bool(*flag),
        Value::Number(value) => Node::Number(number(value)),
        Value::String(text) => string(text),
        Value::Array(items) => Node::Array(items.iter().map(node).collect()),
        Value::Object(map) if map.contains_key(RECORD_ID_KEY) => Node::Reference {
            guid: text_of(map, RECORD_ID_KEY),
            path: text_of(map, RECORD_PATH_KEY),
            embedded: map.keys().any(|key| !LINK_KEYS.contains(&key.as_str())),
        },
        Value::Object(map) => Node::Struct {
            type_name: text_of(map, TYPE_KEY),
            fields: sorted(
                map.iter()
                    .filter(|(key, _)| key.as_str() != TYPE_KEY && key.as_str() != POINTER_KEY)
                    .map(|(key, value)| (key.clone(), node(value)))
                    .collect(),
            ),
        },
    }
}

/// Reads the text of one `StarBreaker` Record file.
///
/// # Errors
///
/// Returns what is wrong with the text: invalid JSON, or a Record without GUID or root struct.
pub(crate) fn record(text: &str) -> Result<RecordDoc, String> {
    let value: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let guid = value
        .get(RECORD_ID_KEY)
        .and_then(Value::as_str)
        .ok_or("no Record GUID")?;
    let root = value
        .get(RECORD_VALUE_KEY)
        .map(node)
        .ok_or("no root struct")?;
    if !matches!(root, Node::Struct { .. }) {
        return Err("the root value is not a struct".to_owned());
    }
    Ok(RecordDoc {
        guid: guid.to_owned(),
        name: value
            .get(RECORD_NAME_KEY)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        tag: value
            .get(RECORD_TAG_KEY)
            .and_then(Value::as_str)
            .map(str::to_owned),
        root,
    })
}
