// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::CorruptChunk;
use crate::streams::{element, u16_at};
use crate::vector::finite_uv;

/// The second texture coordinates of a mesh: two half floats per vertex in the four-byte
/// `NORMALS2` stream.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SecondUvStream<'a> {
    data: &'a [u8],
}

impl<'a> SecondUvStream<'a> {
    const STRIDE: usize = 4;

    pub(crate) const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Returns the second texture coordinate of vertex `index`, with a non-finite value read as 0,
    /// and whether one was.
    pub(crate) fn uv(&self, index: usize) -> Result<([f32; 2], bool), CorruptChunk> {
        let bytes = element(self.data, index, Self::STRIDE, "second texture coordinate")?;
        Ok(finite_uv([0, 2].map(|at| u16_at(bytes, at))))
    }
}
