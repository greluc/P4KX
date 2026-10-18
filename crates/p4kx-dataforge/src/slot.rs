use crate::dataforge::out_of_range;
use crate::rows::PropertyDef;
use crate::schema::CONVERSION_ATTRIBUTE;
use crate::{DataForge, DataForgeError, DataType, Guid, Value, ValueError};

const ARRAY_LEN: u64 = 8;
const TEXT: &str = "text";

/// One property of a struct as it is laid out in an instance: its type and whether it is an
/// array.
#[derive(Clone, Copy)]
pub(crate) struct Slot {
    property: usize,
    def: PropertyDef,
    data_type: DataType,
    array: bool,
}

impl Slot {
    /// Returns the slot of the property at `property`.
    pub(crate) fn of(
        forge: &DataForge<'_>,
        struct_index: u32,
        property: usize,
    ) -> Result<Self, DataForgeError> {
        let def = forge.properties.get(property).ok_or_else(|| {
            out_of_range("struct", struct_index as usize, "property", property as u64)
        })?;
        let data_type =
            DataType::from_code(def.data_type).ok_or(DataForgeError::UnknownDataType {
                code: def.data_type,
            })?;
        Ok(Self {
            property,
            def,
            data_type,
            array: def.conversion != CONVERSION_ATTRIBUTE,
        })
    }

    /// Returns the field name.
    pub(crate) fn name<'f>(&self, forge: &'f DataForge<'f>) -> Result<&'f str, DataForgeError> {
        forge.blob.get(self.def.name)
    }

    /// Returns the number of bytes the field takes in an instance; an inline struct takes the
    /// size of the struct its `target` names.
    pub(crate) fn size(&self, forge: &DataForge<'_>) -> Result<u64, DataForgeError> {
        if self.array {
            return Ok(ARRAY_LEN);
        }
        Ok(match self.data_type {
            DataType::Bool | DataType::Int8 | DataType::UInt8 => 1,
            DataType::Int16 | DataType::UInt16 => 2,
            DataType::Int32
            | DataType::UInt32
            | DataType::Single
            | DataType::String
            | DataType::Locale
            | DataType::Enum => 4,
            DataType::Int64
            | DataType::UInt64
            | DataType::Double
            | DataType::StrongPointer
            | DataType::WeakPointer => 8,
            DataType::Guid => 16,
            DataType::Reference => 20,
            DataType::Class => forge
                .structs
                .get(usize::from(self.def.target))
                .map(|target| u64::from(target.size))
                .ok_or_else(|| {
                    out_of_range("property", self.property, "struct", self.def.target.into())
                })?,
        })
    }

    /// Reads the value from `bytes`, the field's bytes of exactly [`Slot::size`].
    pub(crate) fn read<'f>(&self, forge: &'f DataForge<'f>, bytes: &[u8]) -> Value<'f> {
        let data_type = self.data_type;
        if self.array {
            return Value::Error(ValueError::NotRead {
                data_type,
                array: true,
            });
        }
        match data_type {
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
            DataType::Enum if usize::from(self.def.target) >= forge.enums.len() => {
                Value::Error(ValueError::UnknownEnum {
                    index: self.def.target,
                })
            }
            DataType::Enum => text(forge, bytes, Value::Enum),
            DataType::Class
            | DataType::StrongPointer
            | DataType::WeakPointer
            | DataType::Reference => Value::Error(ValueError::NotRead {
                data_type,
                array: false,
            }),
        }
    }
}

fn text<'f>(forge: &'f DataForge<'f>, bytes: &[u8], wrap: fn(&'f str) -> Value<'f>) -> Value<'f> {
    let offset = u32::from_le_bytes(first_bytes(bytes));
    forge.text.get(offset).map_or(
        Value::Error(ValueError::InvalidString {
            table: TEXT,
            offset,
        }),
        wrap,
    )
}

/// Returns the first `N` bytes, zero-filled when fewer are given.
fn first_bytes<const N: usize>(bytes: &[u8]) -> [u8; N] {
    bytes.first_chunk::<N>().copied().unwrap_or([0; N])
}
