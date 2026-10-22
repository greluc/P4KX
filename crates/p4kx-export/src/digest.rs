use std::hash::{Hash, Hasher};

use p4kx_extraction::Value;
use sha2::{Digest as _, Sha256};

/// A 128-bit digest of structural data (SHA-256 cut to 16 bytes), equal exactly when the data
/// hashed is equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct Digest(u128);

impl Digest {
    /// Returns the digest the 16 bytes `bytes` spell.
    pub(crate) const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(u128::from_le_bytes(bytes))
    }
}

/// A [`Hasher`] that feeds every byte into SHA-256.
pub(crate) struct Wide {
    sha: Sha256,
}

impl Wide {
    pub(crate) fn new() -> Self {
        Self { sha: Sha256::new() }
    }

    pub(crate) fn digest(&self) -> Digest {
        let full = self.sha.clone().finalize();
        let mut bytes = [0_u8; 16];
        for (slot, byte) in bytes.iter_mut().zip(full.iter()) {
            *slot = *byte;
        }
        Digest::from_bytes(bytes)
    }
}

impl Hasher for Wide {
    fn write(&mut self, bytes: &[u8]) {
        self.sha.update(bytes);
    }

    fn finish(&self) -> u64 {
        let mut bytes = [0_u8; 8];
        for (slot, byte) in bytes.iter_mut().zip(self.sha.clone().finalize().iter()) {
            *slot = *byte;
        }
        u64::from_le_bytes(bytes)
    }
}

/// Feeds `value` into `state`: scalars by their bytes, structs by type name, field count and
/// fields, arrays by length and elements, References by the GUID they link to. Localisation keys
/// and the fields `skip` names are left out.
///
/// Returns `false` when the value holds an error or a kind this function does not know, so that
/// a caller does not take the digest as proof of equality.
#[expect(
    clippy::match_same_arms,
    reason = "the error arm names the case the wildcard also covers"
)]
pub(crate) fn hash_value<H: Hasher>(
    state: &mut H,
    value: &Value<'_>,
    skip: &dyn Fn(&str) -> bool,
) -> bool {
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
        Value::Reference(reference) => (11_u8, reference.guid()).hash(state),
        Value::Null => 13_u8.hash(state),
        Value::Struct(inner) => {
            let fields: Vec<_> = inner
                .fields()
                .iter()
                .filter(|field| !skip(field.name()))
                .collect();
            (9_u8, inner.type_name(), fields.len()).hash(state);
            let mut complete = true;
            for field in fields {
                field.name().hash(state);
                complete &= hash_value(state, field.value(), skip);
            }
            return complete;
        }
        Value::Array(items) => {
            (10_u8, items.len()).hash(state);
            let mut complete = true;
            for item in items {
                complete &= hash_value(state, item, skip);
            }
            return complete;
        }
        Value::Error(_) => return false,
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest_of(value: &Value<'_>) -> (Digest, bool) {
        let mut state = Wide::new();
        let complete = hash_value(&mut state, value, &|_| false);
        (state.digest(), complete)
    }

    #[test]
    fn null_and_error_differ_and_an_error_is_incomplete() {
        let (null, null_complete) = digest_of(&Value::Null);
        let (error, error_complete) =
            digest_of(&Value::Error(p4kx_extraction::ValueError::UnknownEnum {
                index: 0,
            }));
        assert_ne!(null, error);
        assert!(null_complete);
        assert!(!error_complete);
    }

    #[test]
    fn arrays_are_prefix_free() {
        let one = Value::Array(Box::new([Value::Array(Box::new([Value::Bool(true)]))]));
        let two = Value::Array(Box::new([Value::Array(Box::new([])), Value::Bool(true)]));
        assert_ne!(digest_of(&one).0, digest_of(&two).0);
    }
}
