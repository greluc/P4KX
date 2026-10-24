// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// An axis-aligned box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// The corner with the smallest coordinates.
    pub min: [f32; 3],
    /// The corner with the largest coordinates.
    pub max: [f32; 3],
}

impl Bounds {
    pub(super) const EMPTY: Self = Self {
        min: [f32::INFINITY; 3],
        max: [f32::NEG_INFINITY; 3],
    };

    pub(super) fn include(&mut self, point: [f32; 3]) {
        if point.iter().all(|value| value.is_finite()) {
            for axis in 0..3 {
                if let (Some(min), Some(max), Some(value)) = (
                    self.min.get_mut(axis),
                    self.max.get_mut(axis),
                    point.get(axis),
                ) {
                    *min = min.min(*value);
                    *max = max.max(*value);
                }
            }
        }
    }

    pub(super) fn union(&mut self, other: &Self) {
        self.include(other.min);
        self.include(other.max);
    }

    pub(super) fn is_valid(&self) -> bool {
        self.min.iter().chain(&self.max).all(|v| v.is_finite())
    }

    /// Returns the centre of the box.
    #[must_use]
    pub fn centre(&self) -> [f32; 3] {
        std::array::from_fn(|axis| {
            let min = self.min.get(axis).copied().unwrap_or(0.0);
            let max = self.max.get(axis).copied().unwrap_or(0.0);
            f32::midpoint(min, max)
        })
    }

    /// Returns the radius of the sphere around the centre that holds the box, never zero.
    #[must_use]
    pub fn radius(&self) -> f32 {
        let squared: f32 = (0..3)
            .map(|axis| {
                let min = self.min.get(axis).copied().unwrap_or(0.0);
                let max = self.max.get(axis).copied().unwrap_or(0.0);
                let half = 0.5 * (max - min);
                half * half
            })
            .sum();
        squared.sqrt().max(1.0e-3)
    }
}
