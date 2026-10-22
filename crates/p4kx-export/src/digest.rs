use std::hash::{DefaultHasher, Hash, Hasher};

use p4kx_extraction::Value;

/// A 128-bit digest of structural data, equal exactly when the data hashed is equal (up to a
/// collision of 2^-128 order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct Digest(u128);

/// Two SipHash states fed the same bytes; the second starts salted.
pub(crate) struct Wide {
    low: DefaultHasher,
    high: DefaultHasher,
}

impl Wide {
    pub(crate) fn new() -> Self {
        let mut high = DefaultHasher::new();
        high.write_u8(0x5a);
        Self {
            low: DefaultHasher::new(),
            high,
        }
    }

    pub(crate) fn digest(&self) -> Digest {
        Digest((u128::from(self.high.finish()) << 64) | u128::from(self.low.finish()))
    }
}

impl Hasher for Wide {
    fn write(&mut self, bytes: &[u8]) {
        self.low.write(bytes);
        self.high.write(bytes);
    }

    fn finish(&self) -> u64 {
        self.low.finish()
    }
}

/// Feeds `value` into `state`: scalars by their bytes, structs by type name and fields, arrays
/// by length and elements, References by the GUID they link to. Localisation keys and the fields
/// `skip` names are left out.
pub(crate) fn hash_value<H: Hasher>(state: &mut H, value: &Value<'_>, skip: &dyn Fn(&str) -> bool) {
    match value {
        Value::Bool(v) => (0_u8, v).hash(state),
        Value::Int8(v) => (1_u8, i64::from(*v)).hash(state),
        Value::Int16(v) => (1_u8, i64::from(*v)).hash(state),
        Value::Int32(v) => (1_u8, i64::from(*v)).hash(state),
        Value::Int64(v) => (1_u8, v).hash(state),
        Value::UInt8(v) => (2_u8, u64::from(*v)).hash(state),
        Value::UInt16(v) => (2_u8, u64::from(*v)).hash(state),
        Value::UInt32(v) => (2_u8, u64::from(*v)).hash(state),
        Value::UInt64(v) => (2_u8, v).hash(state),
        Value::Single(v) => (3_u8, v.to_bits()).hash(state),
        Value::Double(v) => (4_u8, v.to_bits()).hash(state),
        Value::String(v) => (5_u8, v).hash(state),
        Value::Enum(v) => (6_u8, v).hash(state),
        Value::Guid(v) => (7_u8, v).hash(state),
        Value::Locale(_) => 8_u8.hash(state),
        Value::Struct(inner) => {
            (9_u8, inner.type_name()).hash(state);
            for field in inner.fields() {
                if skip(field.name()) {
                    continue;
                }
                field.name().hash(state);
                hash_value(state, field.value(), skip);
            }
        }
        Value::Array(items) => {
            (10_u8, items.len()).hash(state);
            for item in items {
                hash_value(state, item, skip);
            }
        }
        Value::Reference(reference) => (11_u8, reference.guid()).hash(state),
        _ => 12_u8.hash(state),
    }
}
