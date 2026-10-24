// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;

use crate::Value;
use crate::xml_text::FloatText;

/// Writes a scalar in the text form Record XML uses, unescaped; an error value as
/// `error: <reason>`. A struct is written as its type, a Reference as its GUID, a null as
/// `null`, and an array as its elements in brackets separated by `, `, e.g. `[1.5, null]`.
impl fmt::Display for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(value) => fmt::Display::fmt(value, f),
            Self::Int8(value) => fmt::Display::fmt(value, f),
            Self::Int16(value) => fmt::Display::fmt(value, f),
            Self::Int32(value) => fmt::Display::fmt(value, f),
            Self::Int64(value) => fmt::Display::fmt(value, f),
            Self::UInt8(value) => fmt::Display::fmt(value, f),
            Self::UInt16(value) => fmt::Display::fmt(value, f),
            Self::UInt32(value) => fmt::Display::fmt(value, f),
            Self::UInt64(value) => fmt::Display::fmt(value, f),
            Self::Single(value) => fmt::Display::fmt(&FloatText(*value), f),
            Self::Double(value) => fmt::Display::fmt(&FloatText(*value), f),
            Self::String(text) | Self::Locale(text) | Self::Enum(text) => f.write_str(text),
            Self::Guid(guid) => fmt::Display::fmt(guid, f),
            Self::Struct(structure) => f.write_str(structure.type_name()),
            Self::Array(items) => {
                f.write_str("[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    fmt::Display::fmt(item, f)?;
                }
                f.write_str("]")
            }
            Self::Reference(reference) => fmt::Display::fmt(&reference.guid(), f),
            Self::Null => f.write_str("null"),
            Self::Error(error) => write!(f, "error: {error}"),
        }
    }
}
