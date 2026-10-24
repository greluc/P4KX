// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{DataForge, DataType, Guid, Value, ValueError};

/// Reads a scalar of `data_type` from its stored bytes, or returns `None` for a data type that
/// is no scalar; `target` is the enum an enum value belongs to.
pub(crate) fn read_scalar<'f>(
    forge: &'f DataForge<'f>,
    data_type: DataType,
    target: u16,
    bytes: &[u8],
) -> Option<Value<'f>> {
    Some(match data_type {
        DataType::Bool => match first_bytes::<1>(bytes) {
            [0] => Value::Bool(false),
            [1] => Value::Bool(true),
            [byte] => Value::Error(ValueError::InvalidBool { byte }),
        },
        DataType::Int8 => Value::Int8(i8::from_le_bytes(first_bytes(bytes))),
        DataType::Int16 => Value::Int16(i16::from_le_bytes(first_bytes(bytes))),
        DataType::Int32 => Value::Int32(i32::from_le_bytes(first_bytes(bytes))),
        DataType::Int64 => Value::Int64(i64::from_le_bytes(first_bytes(bytes))),
        DataType::UInt8 => Value::UInt8(u8::from_le_bytes(first_bytes(bytes))),
        DataType::UInt16 => Value::UInt16(u16::from_le_bytes(first_bytes(bytes))),
        DataType::UInt32 => Value::UInt32(u32::from_le_bytes(first_bytes(bytes))),
        DataType::UInt64 => Value::UInt64(u64::from_le_bytes(first_bytes(bytes))),
        DataType::Single => Value::Single(f32::from_le_bytes(first_bytes(bytes))),
        DataType::Double => Value::Double(f64::from_le_bytes(first_bytes(bytes))),
        DataType::Guid => Value::Guid(Guid::from_stored(first_bytes(bytes))),
        DataType::String => text(forge, bytes, Value::String),
        DataType::Locale => text(forge, bytes, Value::Locale),
        DataType::Enum if usize::from(target) >= forge.enums.len() => {
            Value::Error(ValueError::UnknownEnum { index: target })
        }
        DataType::Enum => text(forge, bytes, Value::Enum),
        DataType::Class | DataType::StrongPointer | DataType::WeakPointer | DataType::Reference => {
            return None;
        }
    })
}

fn text<'f>(forge: &'f DataForge<'f>, bytes: &[u8], wrap: fn(&'f str) -> Value<'f>) -> Value<'f> {
    let offset = u32::from_le_bytes(first_bytes(bytes));
    forge.text.value(offset).map_or_else(Value::Error, wrap)
}

/// Returns the first `N` bytes, zero-filled when fewer are given.
pub(crate) fn first_bytes<const N: usize>(bytes: &[u8]) -> [u8; N] {
    bytes.first_chunk::<N>().copied().unwrap_or([0; N])
}
