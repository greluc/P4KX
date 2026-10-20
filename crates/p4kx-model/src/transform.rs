/// An affine transform as a 3×4 row-major matrix: three rows of rotation and scale with the
/// translation in the last column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// The matrix rows.
    pub rows: [[f32; 4]; 3],
}

type Linear = [[f32; 3]; 3];

fn entry(matrix: &Linear, row: usize, column: usize) -> f32 {
    matrix
        .get(row)
        .and_then(|values| values.get(column))
        .copied()
        .unwrap_or(0.0)
}

fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left.iter()
        .zip(right)
        .fold(0.0, |sum, (&a, b)| a.mul_add(b, sum))
}

/// Returns the cofactor of `matrix` at `row`, `column`; cycling through the other rows and
/// columns in order gives the sign.
fn cofactor(matrix: &Linear, row: usize, column: usize) -> f32 {
    let (row_a, row_b) = ((row + 1) % 3, (row + 2) % 3);
    let (column_a, column_b) = ((column + 1) % 3, (column + 2) % 3);
    entry(matrix, row_a, column_a).mul_add(
        entry(matrix, row_b, column_b),
        -(entry(matrix, row_a, column_b) * entry(matrix, row_b, column_a)),
    )
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

    fn linear(&self) -> Linear {
        self.rows.map(|[x, y, z, _]| [x, y, z])
    }

    /// Returns the transform that applies `other` first and `self` second.
    pub(crate) fn after(&self, other: &Self) -> Self {
        let (left, right) = (self.linear(), other.linear());
        let (own, moved) = (self.translation(), other.translation());
        let mut rows = [[0.0; 4]; 3];
        for (index, (out, row)) in rows.iter_mut().zip(left).enumerate() {
            for (column, slot) in out.iter_mut().take(3).enumerate() {
                *slot = dot(row, std::array::from_fn(|k| entry(&right, k, column)));
            }
            if let Some(last) = out.get_mut(3) {
                *last = dot(row, moved) + own.get(index).copied().unwrap_or(0.0);
            }
        }
        Self { rows }
    }

    /// Returns the inverse, or `None` when the matrix is singular or not finite.
    pub(crate) fn inverse(&self) -> Option<Self> {
        let linear = self.linear();
        let determinant = dot(
            std::array::from_fn(|column| entry(&linear, 0, column)),
            std::array::from_fn(|column| cofactor(&linear, 0, column)),
        );
        if !determinant.is_finite() || determinant.abs() < f32::EPSILON {
            return None;
        }
        let scale = determinant.recip();
        let inverse: Linear = std::array::from_fn(|row| {
            std::array::from_fn(|column| cofactor(&linear, column, row) * scale)
        });
        let translation = self.translation();
        let rows: [[f32; 4]; 3] = std::array::from_fn(|row| {
            let values = inverse.get(row).copied().unwrap_or_default();
            let [x, y, z] = values;
            [x, y, z, -dot(values, translation)]
        });
        rows.iter()
            .flatten()
            .all(|value| value.is_finite())
            .then_some(Self { rows })
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}
