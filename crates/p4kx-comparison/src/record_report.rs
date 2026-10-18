use std::borrow::Cow;
use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Serialize, Serializer};

use crate::{
    Comparison, FieldChange, Guid, RecordChange, RecordChangeKind, RecordCounts, RecordState,
    Value, ValueError,
};

/// The Records added, removed, moved and changed.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct RecordLevel<'a> {
    /// The number of Records per change kind.
    counts: RecordCounts,
    /// Every added, removed, moved or changed Record, ordered by record path ignoring case and
    /// separator kind, then by GUID.
    changes: Vec<RecordChangeReport<'a>>,
}

/// One added, removed, moved or changed Record.
#[derive(Debug, Serialize, JsonSchema)]
struct RecordChangeReport<'a> {
    /// The GUID that joins both sides.
    #[serde(serialize_with = "display")]
    #[schemars(with = "GuidSchema")]
    guid: Guid,
    /// How the Record differs.
    change: RecordChangeKind,
    /// The Record on the old side, absent when it was added.
    #[serde(skip_serializing_if = "Option::is_none")]
    old: Option<RecordStateReport<'a>>,
    /// The Record on the new side, absent when it was removed.
    #[serde(skip_serializing_if = "Option::is_none")]
    new: Option<RecordStateReport<'a>>,
    /// The changed fields in field order of the new side, the fields only the old side has
    /// last; empty for an added or removed Record.
    fields: Vec<FieldChangeReport<'a>>,
}

/// What identifies a Record on one side.
#[derive(Debug, Serialize, JsonSchema)]
struct RecordStateReport<'a> {
    /// The record path, without `.xml`.
    record_path: &'a str,
    /// The Record name, e.g. `EntityClassDefinition.AEGS_Avenger`.
    name: &'a str,
    /// The name of the Record's struct.
    #[serde(rename = "type")]
    type_name: &'a str,
}

/// One field whose value differs, or that only one side has.
#[derive(Debug, Serialize, JsonSchema)]
struct FieldChangeReport<'a> {
    /// The path of the field in the Record: its name for a field of the Record itself.
    path: &'a str,
    /// The old value, absent when the old side has no such field.
    #[serde(skip_serializing_if = "Option::is_none")]
    old: Option<ValueReport<'a>>,
    /// The new value, absent when the new side has no such field.
    #[serde(skip_serializing_if = "Option::is_none")]
    new: Option<ValueReport<'a>>,
}

/// A scalar value with its type.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum ValueReport<'a> {
    /// A boolean.
    Bool(bool),
    /// A signed 8-bit integer.
    Int8(i8),
    /// A signed 16-bit integer.
    Int16(i16),
    /// A signed 32-bit integer.
    Int32(i32),
    /// A signed 64-bit integer.
    Int64(i64),
    /// An unsigned 8-bit integer.
    #[serde(rename = "uint8")]
    UInt8(u8),
    /// An unsigned 16-bit integer.
    #[serde(rename = "uint16")]
    UInt16(u16),
    /// An unsigned 32-bit integer.
    #[serde(rename = "uint32")]
    UInt32(u32),
    /// An unsigned 64-bit integer.
    #[serde(rename = "uint64")]
    UInt64(u64),
    /// A 32-bit float; `NaN`, `INF` and `-INF` as strings.
    Single(
        #[serde(serialize_with = "single")]
        #[schemars(with = "FloatSchema")]
        f32,
    ),
    /// A 64-bit float; `NaN`, `INF` and `-INF` as strings.
    Double(
        #[serde(serialize_with = "double")]
        #[schemars(with = "FloatSchema")]
        f64,
    ),
    /// A string.
    String(&'a str),
    /// A localisation key, e.g. `@item_Name`.
    Locale(&'a str),
    /// A GUID.
    Guid(
        #[serde(serialize_with = "display")]
        #[schemars(with = "GuidSchema")]
        Guid,
    ),
    /// The name of an enum option.
    Enum(&'a str),
    /// A value that cannot be read, as the reason.
    Error(
        #[serde(serialize_with = "display")]
        #[schemars(with = "String")]
        ValueError,
    ),
}

impl<'a> ValueReport<'a> {
    const fn of(value: &Value<'a>) -> Option<Self> {
        Some(match *value {
            Value::Bool(value) => Self::Bool(value),
            Value::Int8(value) => Self::Int8(value),
            Value::Int16(value) => Self::Int16(value),
            Value::Int32(value) => Self::Int32(value),
            Value::Int64(value) => Self::Int64(value),
            Value::UInt8(value) => Self::UInt8(value),
            Value::UInt16(value) => Self::UInt16(value),
            Value::UInt32(value) => Self::UInt32(value),
            Value::UInt64(value) => Self::UInt64(value),
            Value::Single(value) => Self::Single(value),
            Value::Double(value) => Self::Double(value),
            Value::String(value) => Self::String(value),
            Value::Locale(value) => Self::Locale(value),
            Value::Guid(value) => Self::Guid(value),
            Value::Enum(value) => Self::Enum(value),
            Value::Error(error) => Self::Error(error),
            _ => return None,
        })
    }
}

/// The schema of a GUID in text.
struct GuidSchema;

impl JsonSchema for GuidSchema {
    fn schema_name() -> Cow<'static, str> {
        "Guid".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "A GUID of the form xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx.",
            "type": "string",
            "pattern": "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
        })
    }
}

/// The schema of a float: a number, or `NaN`, `INF` or `-INF`.
struct FloatSchema;

impl JsonSchema for FloatSchema {
    fn schema_name() -> Cow<'static, str> {
        "Float".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "A float: a number, or NaN, INF or -INF.",
            "oneOf": [
                { "type": "number" },
                { "type": "string", "enum": ["NaN", "INF", "-INF"] }
            ]
        })
    }
}

fn display<T: fmt::Display, S: Serializer>(value: &T, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde passes the field by reference"
)]
fn single<S: Serializer>(value: &f32, serializer: S) -> Result<S::Ok, S::Error> {
    match non_finite(f64::from(*value)) {
        Some(text) => serializer.serialize_str(text),
        None => serializer.serialize_f32(*value),
    }
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde passes the field by reference"
)]
fn double<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
    match non_finite(*value) {
        Some(text) => serializer.serialize_str(text),
        None => serializer.serialize_f64(*value),
    }
}

const fn non_finite(value: f64) -> Option<&'static str> {
    if value.is_nan() {
        Some("NaN")
    } else if value.is_infinite() {
        Some(if value.is_sign_negative() {
            "-INF"
        } else {
            "INF"
        })
    } else {
        None
    }
}

impl<'a> From<RecordState<'a>> for RecordStateReport<'a> {
    fn from(state: RecordState<'a>) -> Self {
        Self {
            record_path: state.record_path(),
            name: state.name(),
            type_name: state.type_name(),
        }
    }
}

impl<'a> From<&FieldChange<'a>> for FieldChangeReport<'a> {
    fn from(change: &FieldChange<'a>) -> Self {
        Self {
            path: change.path(),
            old: change.old_value().and_then(ValueReport::of),
            new: change.new_value().and_then(ValueReport::of),
        }
    }
}

impl<'a> From<&RecordChange<'a>> for RecordChangeReport<'a> {
    fn from(change: &RecordChange<'a>) -> Self {
        Self {
            guid: change.guid(),
            change: change.kind(),
            old: change.old_record().map(RecordStateReport::from),
            new: change.new_record().map(RecordStateReport::from),
            fields: change
                .fields()
                .iter()
                .map(FieldChangeReport::from)
                .collect(),
        }
    }
}

pub(crate) fn record_level<'a>(comparison: &'a Comparison<'_>) -> RecordLevel<'a> {
    RecordLevel {
        counts: comparison.record_counts(),
        changes: comparison
            .record_changes()
            .iter()
            .map(RecordChangeReport::from)
            .collect(),
    }
}
