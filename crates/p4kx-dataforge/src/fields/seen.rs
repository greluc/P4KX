use crate::fields::number::{Number, fingerprint, text_fingerprint};
use crate::scalar::first_bytes;
use crate::{DataType, Guid, Value};

/// What one scalar adds to the statistics of its path.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Seen {
    Empty,
    Unreadable,
    Value {
        number: Option<Number>,
        fingerprint: u64,
    },
}

impl Seen {
    /// Returns what the scalar `value` of `data_type` adds: nothing for an empty text or a null
    /// GUID, a fingerprint for every other value and a number for the numeric ones.
    pub(crate) fn of(data_type: DataType, value: &Value<'_>) -> Self {
        let tag = data_type as u8;
        let integer = |value: i128| Self::Value {
            number: Some(Number::Integer(value)),
            fingerprint: wide(tag, value),
        };
        match value {
            Value::Bool(flag) => Self::Value {
                number: None,
                fingerprint: fingerprint(tag, u64::from(*flag)),
            },
            Value::Int8(value) => integer(i128::from(*value)),
            Value::Int16(value) => integer(i128::from(*value)),
            Value::Int32(value) => integer(i128::from(*value)),
            Value::Int64(value) => integer(i128::from(*value)),
            Value::UInt8(value) => integer(i128::from(*value)),
            Value::UInt16(value) => integer(i128::from(*value)),
            Value::UInt32(value) => integer(i128::from(*value)),
            Value::UInt64(value) => integer(i128::from(*value)),
            Value::Single(value) => float(tag, Number::single(*value)),
            Value::Double(value) => float(tag, Number::double(*value)),
            Value::String(text) | Value::Locale(text) | Value::Enum(text) if text.is_empty() => {
                Self::Empty
            }
            Value::String(text) | Value::Locale(text) | Value::Enum(text) => Self::Value {
                number: None,
                fingerprint: text_fingerprint(tag, text),
            },
            Value::Guid(guid) if *guid == Guid::default() => Self::Empty,
            Value::Guid(guid) => Self::Value {
                number: None,
                fingerprint: halves(tag, &guid.stored()),
            },
            _ => Self::Unreadable,
        }
    }
}

fn float(tag: u8, number: Option<Number>) -> Seen {
    Seen::Value {
        number,
        fingerprint: fingerprint(tag, number.map_or(u64::MAX, Number::float_bits)),
    }
}

fn wide(tag: u8, value: i128) -> u64 {
    halves(tag, &value.to_le_bytes())
}

/// Fingerprints 16 bytes as the two words they hold.
fn halves(tag: u8, bytes: &[u8; 16]) -> u64 {
    let low = u64::from_le_bytes(first_bytes(bytes));
    let high = u64::from_le_bytes(first_bytes(bytes.get(8..).unwrap_or_default()));
    fingerprint(tag, low.wrapping_add(fingerprint(0xA5, high)))
}
