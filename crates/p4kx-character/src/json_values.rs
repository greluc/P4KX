use std::fmt;

use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serializer};
use serde_json::value::RawValue;

use crate::CHF_SIZE;
use crate::guid::hex_digit;

/// Bytes as a lowercase hexadecimal string, at most one CHF file long.
pub(crate) mod hex {
    use std::fmt::Write as _;

    use super::{CHF_SIZE, Deserializer, HexVisitor, Serializer};

    pub(crate) fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        let mut text = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            let _infallible = write!(text, "{byte:02x}");
        }
        serializer.serialize_str(&text)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<u8>, D::Error> {
        deserializer.deserialize_str(HexVisitor { max: CHF_SIZE })
    }
}

struct HexVisitor {
    max: usize,
}

impl Visitor<'_> for HexVisitor {
    type Value = Vec<u8>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a string of at most {} hexadecimal digit pairs",
            self.max
        )
    }

    fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Vec<u8>, E> {
        let digits = text.as_bytes();
        if !digits.len().is_multiple_of(2) || digits.len() / 2 > self.max {
            return Err(E::invalid_length(digits.len(), &self));
        }
        digits
            .as_chunks::<2>()
            .0
            .iter()
            .map(|[high, low]| {
                hex_digit(*high)
                    .zip(hex_digit(*low))
                    .map(|(high, low)| high << 4 | low)
            })
            .collect::<Option<Vec<u8>>>()
            .ok_or_else(|| E::invalid_value(serde::de::Unexpected::Str(text), &self))
    }
}

/// An `f32` as a JSON number, or as the string `0x` and its eight-digit bit pattern when it is not
/// finite, read back with exactly the bits it was written with.
pub(crate) mod float {
    use serde::de::Error as _;

    use super::{Deserialize, Deserializer, RawValue, Serializer};

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde calls with-modules by reference"
    )]
    pub(crate) fn serialize<S: Serializer>(value: &f32, serializer: S) -> Result<S::Ok, S::Error> {
        if value.is_finite() {
            serializer.serialize_f32(*value)
        } else {
            serializer.collect_str(&format_args!("0x{:08x}", value.to_bits()))
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f32, D::Error> {
        let raw = <&'de RawValue>::deserialize(deserializer)?;
        let text = raw.get();
        if let Some(quoted) = text.strip_prefix('"') {
            let bits = quoted
                .strip_suffix('"')
                .and_then(|inner| inner.strip_prefix("0x"))
                .filter(|digits| {
                    digits.len() == 8 && digits.bytes().all(|digit| digit.is_ascii_hexdigit())
                })
                .and_then(|digits| u32::from_str_radix(digits, 16).ok())
                .ok_or_else(|| {
                    D::Error::custom(format_args!("{text} is not a float bit pattern 0xXXXXXXXX"))
                })?;
            return Ok(f32::from_bits(bits));
        }
        text.parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| D::Error::custom(format_args!("{text} is not a finite 32-bit float")))
    }
}
