use std::fmt;

use thiserror::Error;

/// The AES-128-CBC IV of encrypted Entries: all zeros.
pub const ARCHIVE_IV: [u8; 16] = [0; 16];

/// The AES-128 key of the encrypted Entries of an Archive, which the user supplies.
///
/// The type never prints its bytes: `Debug` is redacted and there is no `Display`.
#[derive(Clone, PartialEq, Eq)]
pub struct ArchiveKey([u8; ArchiveKey::LEN]);

/// Why text is not an [`ArchiveKey`]; it never repeats the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ArchiveKeyError {
    /// The text is not 32 hexadecimal digits long.
    #[error(
        "an Archive key has {} hexadecimal digits, not {digits}",
        ArchiveKey::HEX_LEN
    )]
    WrongLength {
        /// The number of characters found, whitespace around the text excluded.
        digits: usize,
    },
    /// A character of the text is not a hexadecimal digit.
    #[error("character {position} of the Archive key is not a hexadecimal digit")]
    NotHexadecimal {
        /// The 1-based position of the first such character.
        position: usize,
    },
}

impl ArchiveKey {
    /// The length of the key in bytes.
    pub const LEN: usize = 16;
    /// The length of the key written as hexadecimal digits.
    pub const HEX_LEN: usize = Self::LEN * 2;

    /// Creates a key from its bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes)
    }

    /// Parses 32 hexadecimal digits, upper or lower case, with whitespace around them ignored.
    ///
    /// # Errors
    ///
    /// Returns an [`ArchiveKeyError`] for any other text.
    pub fn from_hex(text: &str) -> Result<Self, ArchiveKeyError> {
        let text = text.trim();
        let digits = text.chars().count();
        if digits != Self::HEX_LEN {
            return Err(ArchiveKeyError::WrongLength { digits });
        }
        let mut bytes = [0_u8; Self::LEN];
        let mut values = text.chars().enumerate().map(|(index, character)| {
            character
                .to_digit(16)
                .and_then(|value| u8::try_from(value).ok())
                .ok_or(ArchiveKeyError::NotHexadecimal {
                    position: index + 1,
                })
        });
        for byte in &mut bytes {
            let (Some(high), Some(low)) = (values.next(), values.next()) else {
                break;
            };
            *byte = (high? << 4) | low?;
        }
        Ok(Self(bytes))
    }

    /// Returns the key bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; Self::LEN] {
        &self.0
    }

    /// Writes the key as 32 lower-case hexadecimal digits.
    #[must_use]
    pub fn to_hex(&self) -> String {
        let mut text = String::with_capacity(Self::HEX_LEN);
        for byte in self.0 {
            for nibble in [byte >> 4, byte & 0x0F] {
                text.push(char::from_digit(u32::from(nibble), 16).unwrap_or('0'));
            }
        }
        text
    }

    /// Returns the key with all but its last four digits hidden, for showing that one is stored.
    #[must_use]
    pub fn masked(&self) -> String {
        let hex = self.to_hex();
        let visible = hex.get(Self::HEX_LEN - 4..).unwrap_or_default();
        format!("{}{visible}", "*".repeat(Self::HEX_LEN - 4))
    }
}

impl fmt::Debug for ArchiveKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ArchiveKey(..)")
    }
}

#[cfg(test)]
mod tests {
    use super::{ArchiveKey, ArchiveKeyError};

    const SAMPLE: &str = "000102030405060708090a0b0c0d0e0f";

    #[test]
    fn hex_digits_of_either_case_parse_and_round_trip() {
        let key = ArchiveKey::from_hex(SAMPLE).expect("lower case");
        let upper = ArchiveKey::from_hex(&SAMPLE.to_uppercase()).expect("upper case");
        assert_eq!(key, upper, "case does not matter");
        assert_eq!(key.as_bytes().last(), Some(&0x0F), "last byte");
        assert_eq!(key.to_hex(), SAMPLE, "round trip");
    }

    #[test]
    fn whitespace_around_the_digits_is_ignored() {
        assert!(
            ArchiveKey::from_hex(&format!("  {SAMPLE}\r\n")).is_ok(),
            "trimmed"
        );
    }

    #[test]
    fn a_wrong_length_is_a_typed_error() {
        assert_eq!(
            ArchiveKey::from_hex("abcd"),
            Err(ArchiveKeyError::WrongLength { digits: 4 }),
            "short"
        );
        assert_eq!(
            ArchiveKey::from_hex(&format!("{SAMPLE}00")),
            Err(ArchiveKeyError::WrongLength { digits: 34 }),
            "long"
        );
    }

    #[test]
    fn a_non_hexadecimal_character_is_a_typed_error() {
        let text = SAMPLE.replacen('5', "g", 1);
        assert_eq!(
            ArchiveKey::from_hex(&text),
            Err(ArchiveKeyError::NotHexadecimal { position: 12 }),
            "position"
        );
    }

    #[test]
    fn the_error_text_never_repeats_the_input() {
        let text = SAMPLE.replacen('5', "g", 1);
        let error = ArchiveKey::from_hex(&text)
            .expect_err("not hex")
            .to_string();
        assert!(!error.contains("0102"), "no digits in {error}");
    }

    #[test]
    fn debug_and_mask_hide_the_key() {
        let key = ArchiveKey::from_hex(SAMPLE).expect("key");
        assert_eq!(format!("{key:?}"), "ArchiveKey(..)", "debug");
        assert_eq!(key.masked(), format!("{}0e0f", "*".repeat(28)), "mask");
    }
}
