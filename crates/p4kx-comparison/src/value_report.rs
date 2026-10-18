use std::borrow::Cow;
use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Serialize, Serializer};

use crate::{ChangedValue, Guid, Location, Reference, Value, ValueError};

/// One side's value of a changed field or array element, with its type.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub(crate) enum ValueReport<'a> {
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
    /// A link to a Record, or for a weak pointer to a place inside the Record.
    Reference(ReferenceReport<'a>),
    /// A struct, as the name of its type; its fields follow as changes of their own.
    Struct(&'a str),
    /// An array, as its number of elements; its elements follow as changes of their own.
    Array(usize),
    /// A null pointer or a null Reference.
    Null,
    /// A value that cannot be read, as the reason.
    Error(
        #[serde(serialize_with = "display")]
        #[schemars(with = "String")]
        ValueError,
    ),
}

/// The target of a Reference.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct ReferenceReport<'a> {
    /// The GUID of the target Record.
    #[serde(serialize_with = "display")]
    #[schemars(with = "GuidSchema")]
    guid: Guid,
    /// The record path of the target Record, without `.xml`.
    record_path: &'a str,
    /// The type of the target: the struct of the Record, or of the instance at `at`.
    #[serde(rename = "type")]
    type_name: &'a str,
    /// For a weak pointer, where inside the target Record it points, e.g. `wheels[0]`; absent
    /// for the Record itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    at: Option<&'a str>,
}

impl<'a> From<&'a Reference<'_>> for ReferenceReport<'a> {
    fn from(link: &'a Reference<'_>) -> Self {
        Self {
            guid: link.guid(),
            record_path: link.record_path(),
            type_name: link.type_name(),
            at: link.location().map(Location::as_str),
        }
    }
}

impl<'a> ValueReport<'a> {
    /// Returns the report of `value`, `None` for a kind of value the report does not know.
    pub(crate) fn of(value: &'a ChangedValue<'_>) -> Option<Self> {
        let value = match value {
            ChangedValue::Struct(type_name) => return Some(Self::Struct(type_name)),
            ChangedValue::Array(len) => return Some(Self::Array(*len)),
            ChangedValue::Value(value) => value,
        };
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
            Value::Reference(ref link) => Self::Reference(ReferenceReport::from(&**link)),
            Value::Null => Self::Null,
            Value::Error(error) => Self::Error(error),
            _ => return None,
        })
    }
}

/// The schema of a GUID in text.
pub(crate) struct GuidSchema;

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

pub(crate) fn display<T: fmt::Display, S: Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
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
