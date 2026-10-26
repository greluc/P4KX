// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::cmp::Ordering;
use std::fmt;

/// The most distinct values counted per path; more only set [`FieldStat::distinct_capped`].
///
/// [`FieldStat::distinct_capped`]: crate::FieldStat::distinct_capped
pub const MAX_DISTINCT_VALUES: usize = 32;

/// A numeric value of a field: an integer, or a float in the width it is stored in.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Number {
    /// Any integer type.
    Integer(i128),
    /// A 32-bit float.
    Single(f32),
    /// A 64-bit float.
    Double(f64),
}

impl Number {
    /// Returns a float with `-0.0` turned into `0.0`, or `None` for NaN.
    pub(crate) fn single(value: f32) -> Option<Self> {
        let zero = value.to_bits() << 1 == 0;
        (!value.is_nan()).then_some(Self::Single(if zero { 0.0 } else { value }))
    }

    /// Returns a float with `-0.0` turned into `0.0`, or `None` for NaN.
    pub(crate) fn double(value: f64) -> Option<Self> {
        let zero = value.to_bits() << 1 == 0;
        (!value.is_nan()).then_some(Self::Double(if zero { 0.0 } else { value }))
    }

    /// Returns the stored bits of a float, which are equal for equal numbers; 0 for an integer.
    pub(crate) fn float_bits(self) -> u64 {
        match self {
            Self::Integer(_) => 0,
            Self::Single(value) => u64::from(value.to_bits()),
            Self::Double(value) => value.to_bits(),
        }
    }

    fn compare(self, other: Self) -> Ordering {
        match (self, other) {
            (Self::Integer(a), Self::Integer(b)) => a.cmp(&b),
            (a, b) => a.as_f64().total_cmp(&b.as_f64()),
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "mixed integer and float values are only ordered, not computed with"
    )]
    fn as_f64(self) -> f64 {
        match self {
            Self::Integer(value) => value as f64,
            Self::Single(value) => f64::from(value),
            Self::Double(value) => value,
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(value) => value.fmt(f),
            Self::Single(value) => float_text(f, value, f64::from(*value)),
            Self::Double(value) => float_text(f, value, *value),
        }
    }
}

/// The smallest magnitude from which a float prints with an exponent.
const EXPONENT_FROM: f64 = 1e16;
/// The magnitude below which a nonzero float prints with an exponent.
const EXPONENT_BELOW: f64 = 1e-5;

/// Prints a float in full, or with an exponent where it is very large or very small, so that
/// `1e30` does not print as thirty digits.
fn float_text<T: fmt::Display + fmt::LowerExp>(
    f: &mut fmt::Formatter<'_>,
    value: &T,
    magnitude: f64,
) -> fmt::Result {
    let magnitude = magnitude.abs();
    if magnitude.is_finite()
        && (magnitude >= EXPONENT_FROM || (magnitude != 0.0 && magnitude < EXPONENT_BELOW))
    {
        fmt::LowerExp::fmt(value, f)
    } else {
        fmt::Display::fmt(value, f)
    }
}

/// The smallest and the largest number seen.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Extremes {
    pub(crate) min: Number,
    pub(crate) max: Number,
}

impl Extremes {
    pub(crate) const fn of(value: Number) -> Self {
        Self {
            min: value,
            max: value,
        }
    }

    pub(crate) fn add(&mut self, value: Number) {
        if value.compare(self.min) == Ordering::Less {
            self.min = value;
        }
        if value.compare(self.max) == Ordering::Greater {
            self.max = value;
        }
    }

    pub(crate) fn merge(&mut self, other: Self) {
        self.add(other.min);
        self.add(other.max);
    }
}

/// The 64-bit fingerprints of up to [`MAX_DISTINCT_VALUES`] distinct values, or the note that
/// there were more.
#[derive(Debug, Clone, Default)]
pub(crate) struct Distinct {
    seen: Vec<u64>,
    capped: bool,
}

impl Distinct {
    pub(crate) fn add(&mut self, fingerprint: u64) {
        if self.capped || self.seen.contains(&fingerprint) {
            return;
        }
        if self.seen.len() == MAX_DISTINCT_VALUES {
            self.capped = true;
            self.seen = Vec::new();
        } else {
            self.seen.push(fingerprint);
        }
    }

    pub(crate) fn merge(&mut self, other: Self) {
        if other.capped {
            self.capped = true;
            self.seen = Vec::new();
        }
        for fingerprint in other.seen {
            self.add(fingerprint);
        }
    }

    pub(crate) const fn count(&self) -> usize {
        if self.capped {
            MAX_DISTINCT_VALUES
        } else {
            self.seen.len()
        }
    }

    pub(crate) const fn is_capped(&self) -> bool {
        self.capped
    }
}

/// Mixes a value and the tag of its kind into a fingerprint.
pub(crate) const fn fingerprint(tag: u8, value: u64) -> u64 {
    let mut x = value ^ (tag as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Returns the FNV-1a hash of `text`, fingerprinted with the tag of its kind.
pub(crate) fn text_fingerprint(tag: u8, text: &str) -> u64 {
    let hash = text.bytes().fold(0xCBF2_9CE4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3)
    });
    fingerprint(tag, hash)
}
