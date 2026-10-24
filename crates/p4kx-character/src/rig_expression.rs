// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::rig::SLOT_EXPRESSION_MORPHS;
use crate::rig_bytes::{u32_at, vec3_at};
use crate::{DnaRig, RigError};

const SLOT: usize = SLOT_EXPRESSION_MORPHS;
const WHAT: &str = "expression morphs";
const RECORD_LEN: usize = 32;
const ENTRY_LEN: usize = 4;
const SCALE_AT: usize = 8;
const BIAS_AT: usize = 20;

/// The blendshapes of the heads of a rig: for every head and every blendshape of the rig, the
/// vertices the blendshape moves on that head and by how much.
///
/// The cell of head 0 and blendshape `b` is the blendshape `b` of the base head. Read with
/// [`DnaRig::expression_morphs`].
#[derive(Debug, Clone, Copy)]
pub struct ExpressionMorphs<'a> {
    data: &'a [u8],
    variants: usize,
    blendshapes: usize,
}

/// The vertices one blendshape moves on one head, see [`ExpressionMorphs::cell`].
#[derive(Debug, Clone, Copy)]
pub struct ExpressionCell<'a> {
    start_vertex: u32,
    scale: [f32; 3],
    bias: [f32; 3],
    entries: &'a [u8],
}

impl<'a> DnaRig<'a> {
    /// Reads the table of the blendshapes of each head.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::SlotUnset`], [`RigError::SlotTooShort`] or [`RigError::Malformed`] when
    /// the table is missing or its cells do not divide evenly between the heads.
    pub fn expression_morphs(&self) -> Result<ExpressionMorphs<'a>, RigError> {
        let data = self.slot(SLOT, WHAT)?;
        let malformed = |reason| RigError::Malformed {
            what: WHAT,
            slot: SLOT,
            reason,
        };
        let cells_len = u32_at(data, 0)
            .and_then(|len| usize::try_from(len).ok())
            .ok_or_else(|| malformed("the slot is shorter than its length field"))?;
        if cells_len > data.len() {
            return Err(malformed("the cell table is longer than the slot"));
        }
        let cells = cells_len / RECORD_LEN;
        if cells == 0 || !cells.is_multiple_of(self.variants()) {
            return Err(malformed(
                "the cells do not divide evenly between the heads",
            ));
        }
        Ok(ExpressionMorphs {
            data,
            variants: self.variants(),
            blendshapes: cells / self.variants(),
        })
    }
}

impl<'a> ExpressionMorphs<'a> {
    /// Returns the number of heads.
    #[must_use]
    pub const fn variants(&self) -> usize {
        self.variants
    }

    /// Returns the number of blendshapes per head.
    #[must_use]
    pub const fn blendshapes(&self) -> usize {
        self.blendshapes
    }

    /// Returns the cell of `variant` and `blendshape`, or `None` for an empty cell, one outside
    /// the table or one whose entries do not fit the slot.
    #[must_use]
    pub fn cell(&self, variant: usize, blendshape: usize) -> Option<ExpressionCell<'a>> {
        if variant >= self.variants || blendshape >= self.blendshapes {
            return None;
        }
        let at = variant
            .checked_mul(self.blendshapes)?
            .checked_add(blendshape)?
            .checked_mul(RECORD_LEN)?;
        let packed = u32_at(self.data, at.checked_add(4)?)?;
        if packed == 0 {
            return None;
        }
        let count = usize::from(u16::try_from(packed & 0xffff).ok()?);
        let offset = usize::try_from(u32_at(self.data, at)?).ok()?;
        let end = offset.checked_add(count.checked_mul(ENTRY_LEN)?)?;
        Some(ExpressionCell {
            start_vertex: packed >> 16,
            scale: vec3_at(self.data, at.checked_add(SCALE_AT)?)?,
            bias: vec3_at(self.data, at.checked_add(BIAS_AT)?)?,
            entries: self.data.get(offset..end)?,
        })
    }
}

impl ExpressionCell<'_> {
    /// Returns the number of vertices the cell moves.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len() / ENTRY_LEN
    }

    /// Returns whether the cell moves no vertex.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the vertices the cell moves and by how much in metres, in the order of the
    /// vertices.
    pub fn moves(&self) -> impl Iterator<Item = (u32, [f32; 3])> {
        let mut vertex = self.start_vertex;
        self.entries
            .as_chunks::<ENTRY_LEN>()
            .0
            .iter()
            .map(move |&[stride, x, y, z]| {
                vertex = vertex.saturating_add(u32::from(stride));
                let codes = [x, y, z];
                let moved = std::array::from_fn(|axis| {
                    let code = codes.get(axis).copied().unwrap_or_default();
                    let scale = self.scale.get(axis).copied().unwrap_or_default();
                    let bias = self.bias.get(axis).copied().unwrap_or_default();
                    f32::from(code).mul_add(scale, bias)
                });
                (vertex, moved)
            })
    }
}
