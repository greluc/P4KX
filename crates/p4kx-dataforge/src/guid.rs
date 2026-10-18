use std::fmt;
use std::str::FromStr;

use thiserror::Error;

const TEXT_LEN: usize = 36;
const DASHES: [usize; 4] = [8, 13, 18, 23];
const TEXT_ORDER: [usize; 16] = [7, 6, 5, 4, 3, 2, 1, 0, 15, 14, 13, 12, 11, 10, 9, 8];

/// The GUID of a Record or a GUID value, in the byte order DataForge stores it.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
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
            let high = digits.next().and_then(|digit| hex(*digit));
            let low = digits.next().and_then(|digit| hex(*digit));
            let (Some(high), Some(low), Some(slot)) = (high, low, stored.get_mut(index)) else {
                return Err(InvalidGuid);
            };
            *slot = high << 4 | low;
        }
        Ok(Self(stored))
    }
}

const fn hex(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}
