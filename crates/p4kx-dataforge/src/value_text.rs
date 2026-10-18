use std::fmt;

use crate::Value;
use crate::xml_text::FloatText;

/// Writes a scalar in the text form Record XML uses, unescaped; an error value as
/// `error: <reason>`. A nested value is summarised: a struct by its type, an array as
/// `[<count>]`, a Reference by its GUID and a null as `null`.
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
            Self::Array(items) => write!(f, "[{}]", items.len()),
            Self::Reference(reference) => fmt::Display::fmt(&reference.guid(), f),
            Self::Null => f.write_str("null"),
            Self::Error(error) => write!(f, "error: {error}"),
        }
    }
}
