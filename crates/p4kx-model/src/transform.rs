// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// An affine transform as a 3×4 row-major matrix: three rows of rotation and scale with the
/// translation in the last column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// The matrix rows.
    pub rows: [[f32; 4]; 3],
}

impl Transform {
    /// The transform that changes nothing.
    pub const IDENTITY: Self = Self {
        rows: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ],
    };

    /// Returns whether this is the identity transform.
    #[must_use]
    pub fn is_identity(&self) -> bool {
        *self == Self::IDENTITY
    }

    /// Returns the translation.
    #[must_use]
    pub const fn translation(&self) -> [f32; 3] {
        let [[_, _, _, x], [_, _, _, y], [_, _, _, z]] = self.rows;
        [x, y, z]
    }
}

impl Transform {
    /// Returns the transform that turns by the quaternion `rotation`, given as `[w, x, y, z]`
    /// and normalised here, and then moves by `position`; or `None` when a value is not finite or
    /// the quaternion has no length.
    #[must_use]
    #[expect(
        clippy::suboptimal_flops,
        reason = "the matrix entries follow the textbook quaternion formulas"
    )]
    pub fn from_quaternion(rotation: [f64; 4], position: [f64; 3]) -> Option<Self> {
        let [w, x, y, z] = rotation;
        let length = w.mul_add(w, x.mul_add(x, y.mul_add(y, z * z))).sqrt();
        if !length.is_finite() || length < f64::EPSILON || !position.iter().all(|v| v.is_finite()) {
            return None;
        }
        let [w, x, y, z] = [w / length, x / length, y / length, z / length];
        let [px, py, pz] = position;
        Some(Self {
            rows: [
                [
                    narrow((-2.0_f64).mul_add(y * y + z * z, 1.0)),
                    narrow(2.0 * x.mul_add(y, -(w * z))),
                    narrow(2.0 * x.mul_add(z, w * y)),
                    narrow(px),
                ],
                [
                    narrow(2.0 * x.mul_add(y, w * z)),
                    narrow((-2.0_f64).mul_add(x * x + z * z, 1.0)),
                    narrow(2.0 * y.mul_add(z, -(w * x))),
                    narrow(py),
                ],
                [
                    narrow(2.0 * x.mul_add(z, -(w * y))),
                    narrow(2.0 * y.mul_add(z, w * x)),
                    narrow((-2.0_f64).mul_add(x * x + y * y, 1.0)),
                    narrow(pz),
                ],
            ],
        })
    }

    /// Returns the transform of the 3×4 row-major matrix `rows`, or `None` when a value is not
    /// finite once narrowed to single precision.
    #[must_use]
    pub fn from_rows_f64(rows: [[f64; 4]; 3]) -> Option<Self> {
        let rows = rows.map(|row| row.map(narrow));
        rows.iter()
            .flatten()
            .all(|value| value.is_finite())
            .then_some(Self { rows })
    }

    /// Returns the transform that applies `inner` first and this transform after it: the matrix
    /// product `self * inner`, so `parent.then(&child)` places a child in its parent's parent.
    #[must_use]
    pub fn then(&self, inner: &Self) -> Self {
        let outer = self.rows.map(|row| row.map(f64::from));
        let inner = inner.rows.map(|row| row.map(f64::from));
        let rows = std::array::from_fn(|r| {
            std::array::from_fn(|c| {
                let linear: f64 = (0..3).map(|k| at(&outer, r, k) * at(&inner, k, c)).sum();
                let shift = if c == 3 { at(&outer, r, 3) } else { 0.0 };
                narrow(linear + shift)
            })
        });
        Self { rows }
    }

    /// Returns the transform that undoes this one, or `None` when the matrix is singular or not
    /// finite.
    #[must_use]
    pub fn inverse(&self) -> Option<Self> {
        let m = self.rows.map(|row| row.map(f64::from));
        let cofactor = |r: usize, c: usize| {
            let (r1, r2) = ((r + 1) % 3, (r + 2) % 3);
            let (c1, c2) = ((c + 1) % 3, (c + 2) % 3);
            at(&m, r1, c1).mul_add(at(&m, r2, c2), -(at(&m, r1, c2) * at(&m, r2, c1)))
        };
        let determinant: f64 = (0..3).map(|c| at(&m, 0, c) * cofactor(0, c)).sum();
        if !determinant.is_finite() || determinant.abs() < f64::EPSILON {
            return None;
        }
        let linear = |r: usize, c: usize| cofactor(c, r) / determinant;
        let rows = std::array::from_fn(|r| {
            std::array::from_fn(|c| {
                if c < 3 {
                    narrow(linear(r, c))
                } else {
                    let shift: f64 = (0..3).map(|k| linear(r, k) * at(&m, k, 3)).sum();
                    narrow(-shift)
                }
            })
        });
        let inverse = Self { rows };
        inverse
            .rows
            .iter()
            .flatten()
            .all(|v| v.is_finite())
            .then_some(inverse)
    }
}

fn at(rows: &[[f64; 4]; 3], row: usize, column: usize) -> f64 {
    rows.get(row)
        .and_then(|values| values.get(column))
        .copied()
        .unwrap_or(0.0)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "transforms are stored at single precision"
)]
const fn narrow(value: f64) -> f32 {
    value as f32
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}
