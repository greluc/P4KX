// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

const TEXT_LEN: usize = 36;
const DASHES: [usize; 4] = [8, 13, 18, 23];
const TEXT_ORDER: [usize; 16] = [7, 6, 5, 4, 3, 2, 1, 0, 15, 14, 13, 12, 11, 10, 9, 8];

/// A GUID in a CHF body, stored and printed like a DataForge Record GUID.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Guid([u8; 16]);

impl Guid {
    /// Creates a GUID from its 16 stored bytes.
    #[must_use]
    pub const fn from_stored(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Returns the 16 stored bytes.
    #[must_use]
    pub const fn stored(&self) -> [u8; 16] {
        self.0
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (position, index) in TEXT_ORDER.into_iter().enumerate() {
            if matches!(position, 4 | 6 | 8 | 10) {
                f.write_str("-")?;
            }
            let byte = self.0.get(index).copied().unwrap_or_default();
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Guid({self})")
    }
}

/// Text that is not a GUID of the form `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not a GUID of the form xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx")]
pub struct InvalidGuid;

impl FromStr for Guid {
    type Err = InvalidGuid;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.as_bytes();
        if text.len() != TEXT_LEN || DASHES.iter().any(|at| text.get(*at) != Some(&b'-')) {
            return Err(InvalidGuid);
        }
        let mut digits = text.iter().filter(|byte| **byte != b'-');
        let mut stored = [0; 16];
        for index in TEXT_ORDER {
            let high = digits.next().and_then(|digit| hex_digit(*digit));
            let low = digits.next().and_then(|digit| hex_digit(*digit));
            let (Some(high), Some(low), Some(slot)) = (high, low, stored.get_mut(index)) else {
                return Err(InvalidGuid);
            };
            *slot = high << 4 | low;
        }
        Ok(Self(stored))
    }
}

impl Serialize for Guid {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Guid {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(GuidVisitor)
    }
}

struct GuidVisitor;

impl serde::de::Visitor<'_> for GuidVisitor {
    type Value = Guid;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a GUID of the form xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx")
    }

    fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Guid, E> {
        text.parse().map_err(E::custom)
    }
}

/// The value of one hexadecimal digit in either case.
pub(crate) const fn hex_digit(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}
