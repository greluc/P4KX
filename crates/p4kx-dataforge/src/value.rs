use std::fmt;

use thiserror::Error;

use crate::{DataType, Guid};

/// The value of one field in the canonical Record tree.
///
/// Text is borrowed from the DataForge. A value that cannot be read is an [`Value::Error`] at its
/// field, so the rest of the Record still builds.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Value<'f> {
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
    UInt8(u8),
    /// An unsigned 16-bit integer.
    UInt16(u16),
    /// An unsigned 32-bit integer.
    UInt32(u32),
    /// An unsigned 64-bit integer.
    UInt64(u64),
    /// A 32-bit float.
    Single(f32),
    /// A 64-bit float.
    Double(f64),
    /// A string.
    String(&'f str),
    /// A localisation key, kept as the key, e.g. `@item_Name`.
    Locale(&'f str),
    /// A GUID.
    Guid(Guid),
    /// The name of an enum option.
    Enum(&'f str),
    /// A value that cannot be read, with the reason.
    Error(ValueError),
}

/// Why the value of one field cannot be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ValueError {
    /// A string, localisation key or enum value is not a valid string offset.
    #[error("offset {offset} in the {table} table is not a valid string")]
    InvalidString {
        /// The table the offset points into.
        table: &'static str,
        /// The offset.
        offset: u32,
    },
    /// An enum field names an enum that does not exist.
    #[error("the field refers to enum {index}, which does not exist")]
    UnknownEnum {
        /// The enum index the property gives.
        index: u16,
    },
    /// A bool byte is neither 0 nor 1.
    #[error("the bool byte {byte} is neither 0 nor 1")]
    InvalidBool {
        /// The stored byte.
        byte: u8,
    },
    /// The field holds a kind of value this version does not read yet.
    #[error("{} not read yet", NotReadKind { data_type: *data_type, array: *array })]
    NotRead {
        /// The data type of the field.
        data_type: DataType,
        /// Whether the field holds an array.
        array: bool,
    },
}

struct NotReadKind {
    data_type: DataType,
    array: bool,
}

impl fmt::Display for NotReadKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.array {
            write!(f, "arrays of {} are", self.data_type)
        } else {
            write!(f, "{} values are", self.data_type)
        }
    }
}

/// One field of a Record or struct value: its name and its value.
#[derive(Debug, Clone, PartialEq)]
pub struct Field<'f> {
    pub(crate) name: &'f str,
    pub(crate) value: Value<'f>,
}

impl<'f> Field<'f> {
    /// Returns the field name as the struct declares it.
    #[must_use]
    pub const fn name(&self) -> &'f str {
        self.name
    }

    /// Returns the value.
    #[must_use]
    pub const fn value(&self) -> &Value<'f> {
        &self.value
    }
}
