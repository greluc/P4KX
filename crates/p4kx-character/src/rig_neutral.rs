// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::rig::SLOT_NEUTRAL_MORPHS;
use crate::rig_bytes::{u32_at, vec3_at};
use crate::{DnaMatrix, DnaRig, RigError};

const SLOT: usize = SLOT_NEUTRAL_MORPHS;
const WHAT: &str = "neutral morphs";
const RECORD_LEN: usize = 0x20;
const BIAS_AT: usize = 0x10;
const GAP: usize = 4;

/// The neutral face shapes of the heads of a rig.
///
/// For every vertex and every control, which is a head other than the base head in one of the
/// regions the file covers, it says whether the control moves the vertex and by how much.
///
/// A control moves a vertex by `code * scale + bias` per axis, the code a byte of the file and the
/// scale and bias the control's; the Character's weight of the control's head in the control's
/// region scales the move. The base head is the mesh as it is, so it has no control.
///
/// Read with [`DnaRig::neutral_morphs`].
#[derive(Debug, Clone)]
pub struct NeutralMorphs<'a> {
    data: &'a [u8],
    vertices: usize,
    variants: usize,
    regions: usize,
    covered: Vec<usize>,
    controls: usize,
    mask_words: usize,
    cursors_at: usize,
    records_at: usize,
    codes_at: usize,
}

impl<'a> DnaRig<'a> {
    /// Reads the neutral morphs of the mesh this file belongs to.
    ///
    /// The layout is checked against the file: the controls, bit masks, cursors and codes must
    /// fit the slot, and the cursor of every vertex must count the controls the masks of the
    /// vertices before it set.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::SlotUnset`], [`RigError::SlotTooShort`] or [`RigError::Malformed`] when
    /// the morphs are missing or do not fit the layout.
    pub fn neutral_morphs(&self) -> Result<NeutralMorphs<'a>, RigError> {
        let data = self.slot(SLOT, WHAT)?;
        let covered = self.covered_regions()?;
        let vertices = self.vertex_count()?;
        let malformed = |reason| RigError::Malformed {
            what: WHAT,
            slot: SLOT,
            reason,
        };
        let controls = covered
            .len()
            .checked_mul(self.variants().saturating_sub(1))
            .filter(|&controls| controls > 0)
            .ok_or_else(|| malformed("the rig has no head besides the base head"))?;
        let mask_words = controls.div_ceil(32);
        let too_short = |need: Option<usize>| RigError::SlotTooShort {
            what: WHAT,
            slot: SLOT,
            need: need.map_or(u64::MAX, |need| need as u64),
            have: data.len(),
        };
        let cursors_at = vertices
            .checked_mul(mask_words)
            .and_then(|words| words.checked_mul(4));
        let records_at = cursors_at.and_then(|at| at.checked_add(vertices.checked_mul(4)?));
        let codes_at = records_at
            .and_then(|at| at.checked_add(controls.checked_mul(RECORD_LEN)?))
            .and_then(|at| at.checked_add(GAP));
        let (Some(cursors_at), Some(records_at), Some(codes_at)) = (
            cursors_at,
            records_at,
            codes_at.filter(|&at| at <= data.len()),
        ) else {
            return Err(too_short(codes_at));
        };
        let morphs = NeutralMorphs {
            data,
            vertices,
            variants: self.variants(),
            regions: self.regions(),
            covered,
            controls,
            mask_words,
            cursors_at,
            records_at,
            codes_at,
        };
        morphs.check_cursors()?;
        Ok(morphs)
    }
}

impl NeutralMorphs<'_> {
    /// Returns the number of vertices.
    #[must_use]
    pub const fn vertices(&self) -> usize {
        self.vertices
    }

    /// Returns the number of controls: the covered regions times the heads besides the base head.
    #[must_use]
    pub const fn controls(&self) -> usize {
        self.controls
    }

    const fn malformed(reason: &'static str) -> RigError {
        RigError::Malformed {
            what: WHAT,
            slot: SLOT,
            reason,
        }
    }

    fn mask_word(&self, vertex: usize, word: usize) -> u32 {
        vertex
            .checked_mul(self.mask_words)
            .and_then(|row| row.checked_add(word))
            .and_then(|at| at.checked_mul(4))
            .and_then(|at| u32_at(self.data, at))
            .unwrap_or_default()
    }

    fn cursor(&self, vertex: usize) -> Option<usize> {
        let at = vertex.checked_mul(4)?.checked_add(self.cursors_at)?;
        usize::try_from(u32_at(self.data, at)?).ok()
    }

    fn check_cursors(&self) -> Result<(), RigError> {
        let mut expected = 0_usize;
        for vertex in 0..self.vertices {
            if self.cursor(vertex) != Some(expected) {
                return Err(Self::malformed(
                    "a cursor does not count the controls set before its vertex",
                ));
            }
            let set: u32 = (0..self.mask_words)
                .map(|word| self.mask_word(vertex, word).count_ones())
                .sum();
            expected = expected.saturating_add(set as usize);
        }
        let codes = self.data.len().saturating_sub(self.codes_at) / 3;
        if expected > codes {
            return Err(Self::malformed("the codes end before the last vertex's"));
        }
        Ok(())
    }

    /// Returns how far the morphs move each vertex for the weights `matrix`, in metres in the
    /// mesh's space, to be added to the vertex positions.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::MatrixShape`] when the matrix is not of this rig.
    pub fn decode(&self, matrix: &DnaMatrix) -> Result<Vec<[f32; 3]>, RigError> {
        if matrix.variants() != self.variants || matrix.regions() != self.regions {
            return Err(RigError::MatrixShape {
                matrix_variants: matrix.variants(),
                matrix_regions: matrix.regions(),
                variants: self.variants,
                regions: self.regions,
            });
        }
        let mut coefficients = vec![0.0_f32; self.controls];
        for variant in 1..self.variants {
            for (local, &region) in self.covered.iter().enumerate() {
                let control = (variant - 1) * self.covered.len() + local;
                if let Some(slot) = coefficients.get_mut(control) {
                    *slot = matrix.cell(variant, region);
                }
            }
        }
        Ok(self.decode_weights(&coefficients))
    }

    /// Returns how far the morphs move each vertex for one weight per control, in metres, to be
    /// added to the vertex positions; control `(head - 1) * covered + local` belongs to the head
    /// `head` and the covered region number `local`. Missing weights count as 0.
    #[must_use]
    pub fn decode_weights(&self, weights: &[f32]) -> Vec<[f32; 3]> {
        let mut moves = vec![[0.0_f32; 3]; self.vertices];
        for (vertex, total) in moves.iter_mut().enumerate() {
            let cursor = self.cursor(vertex).unwrap_or_default();
            let mut active = 0_usize;
            for word in 0..self.mask_words {
                let mut bits = self.mask_word(vertex, word);
                while bits != 0 {
                    let control = word * 32 + bits.trailing_zeros() as usize;
                    bits &= bits - 1;
                    let at = cursor.saturating_add(active);
                    active += 1;
                    let weight = weights.get(control).copied().unwrap_or_default();
                    if weight != 0.0 && control < self.controls {
                        self.add_move(total, control, at, weight);
                    }
                }
            }
        }
        moves
    }

    fn add_move(&self, total: &mut [f32; 3], control: usize, code: usize, weight: f32) {
        let record = self.records_at + control * RECORD_LEN;
        let codes_at = code
            .checked_mul(3)
            .and_then(|at| at.checked_add(self.codes_at));
        let codes = codes_at.and_then(|at| self.data.get(at..at.checked_add(3)?));
        let (Some(codes), Some(scale), Some(bias)) = (
            codes,
            vec3_at(self.data, record),
            vec3_at(self.data, record + BIAS_AT),
        ) else {
            return;
        };
        for (((total, &code), scale), bias) in total.iter_mut().zip(codes).zip(scale).zip(bias) {
            *total = weight.mul_add(f32::from(code).mul_add(scale, bias), *total);
        }
    }
}
