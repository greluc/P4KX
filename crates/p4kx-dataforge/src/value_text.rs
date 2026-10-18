use std::fmt;

use crate::Value;
use crate::xml_text::FloatText;

/// Writes the value in the text form Record XML uses, unescaped; an error value as
/// `error: <reason>`.
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
            Self::Error(error) => write!(f, "error: {error}"),
        }
    }
}
