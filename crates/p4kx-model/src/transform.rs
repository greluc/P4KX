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

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}
