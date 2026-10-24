// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{Dna, DnaRig, RigError};

/// The most heads blended per region.
const MAX_BLENDED: usize = 4;

/// The most cells (heads times regions) of a [`DnaMatrix`]; a human head has about 800.
pub const MAX_RIG_CELLS: usize = 1 << 16;

/// The weights with which the heads of a rig are blended for a Character, one per head and region.
///
/// Built from the Character's DNA by [`DnaMatrix::from_dna`]: for every region the blends of the
/// DNA are added up and normalised so that the weights of the heads sum to 1, and only the four
/// strongest heads are kept. A region without blends keeps all weights at 0, which leaves the
/// base head unchanged there.
#[derive(Debug, Clone, PartialEq)]
pub struct DnaMatrix {
    variants: usize,
    regions: usize,
    cells: Vec<f32>,
}

impl DnaMatrix {
    /// Returns the matrix of `dna` for a rig of `variants` heads and `regions` regions.
    ///
    /// Each blend adds `value / 65535` to the cell of its head and its face part, which is capped
    /// at 1; blends of value 0 are ignored. Each face part is a region by its index.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::HeadOutOfRange`] for a blend of a head the rig lacks and
    /// [`RigError::FacePartCount`] for blends in a face part the rig has no region for.
    pub fn from_dna(dna: &Dna, variants: usize, regions: usize) -> Result<Self, RigError> {
        if variants.saturating_mul(regions) > MAX_RIG_CELLS {
            return Err(RigError::TooLarge {
                variants,
                regions,
                max: MAX_RIG_CELLS,
            });
        }
        let mut columns = vec![vec![0.0_f64; variants]; regions];
        for (part, blends) in dna.face_parts.iter().enumerate() {
            for blend in blends.iter().filter(|blend| blend.value != 0) {
                let column = columns.get_mut(part).ok_or(RigError::FacePartCount {
                    parts: dna.face_parts.len(),
                    regions,
                })?;
                let cell =
                    column
                        .get_mut(usize::from(blend.head_id))
                        .ok_or(RigError::HeadOutOfRange {
                            head_id: blend.head_id,
                            variants,
                        })?;
                *cell = (*cell + f64::from(blend.value) / 65535.0).clamp(0.0, 1.0);
            }
        }
        let columns: Vec<Vec<f32>> = columns.iter().map(|column| blend_weights(column)).collect();
        let cells = (0..variants)
            .flat_map(|variant| {
                columns
                    .iter()
                    .map(move |column| column.get(variant).copied().unwrap_or_default())
            })
            .collect();
        Ok(Self {
            variants,
            regions,
            cells,
        })
    }

    /// Returns the number of heads.
    #[must_use]
    pub const fn variants(&self) -> usize {
        self.variants
    }

    /// Returns the number of regions.
    #[must_use]
    pub const fn regions(&self) -> usize {
        self.regions
    }

    /// Returns the weight of `variant` in `region`, 0 outside the matrix.
    #[must_use]
    pub fn cell(&self, variant: usize, region: usize) -> f32 {
        if region >= self.regions {
            return 0.0;
        }
        variant
            .checked_mul(self.regions)
            .and_then(|row| row.checked_add(region))
            .and_then(|at| self.cells.get(at))
            .copied()
            .unwrap_or_default()
    }

    /// Checks that the matrix has the shape of `rig`.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::MatrixShape`] when it does not.
    pub const fn check(&self, rig: &DnaRig<'_>) -> Result<(), RigError> {
        if self.variants == rig.variants() && self.regions == rig.regions() {
            Ok(())
        } else {
            Err(RigError::MatrixShape {
                matrix_variants: self.variants,
                matrix_regions: self.regions,
                variants: rig.variants(),
                regions: rig.regions(),
            })
        }
    }
}

/// Turns the summed blend values of the heads in one region into weights that sum to 1, keeping
/// only the [`MAX_BLENDED`] strongest; ties are dropped from the lowest head ID on.
fn blend_weights(summed: &[f64]) -> Vec<f32> {
    let total: f64 = summed.iter().sum();
    let mut weights: Vec<f32> = summed
        .iter()
        .map(|&value| narrow(if total > 0.0 { value / total } else { 0.0 }))
        .collect();
    while weights.iter().filter(|&&weight| weight > 0.0).count() > MAX_BLENDED {
        let weakest = weights
            .iter_mut()
            .filter(|weight| **weight > 0.0)
            .min_by(|a, b| a.total_cmp(b));
        match weakest {
            Some(weight) => *weight = 0.0,
            None => break,
        }
    }
    let kept: f32 = weights.iter().sum();
    if kept > 0.0 {
        for weight in &mut weights {
            *weight /= kept;
        }
    }
    weights
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the weights are in 0..=1, an f32 holds them as the game does"
)]
const fn narrow(value: f64) -> f32 {
    value as f32
}

impl DnaRig<'_> {
    /// Returns the matrix of `dna` for this rig, see [`DnaMatrix::from_dna`].
    ///
    /// # Errors
    ///
    /// Returns what [`DnaMatrix::from_dna`] does.
    pub fn matrix(&self, dna: &Dna) -> Result<DnaMatrix, RigError> {
        DnaMatrix::from_dna(dna, self.variants(), self.regions())
    }

    /// Returns how far the joints of a Character's head sit from the base head's: for each joint,
    /// the sum over the heads of the head's translation minus the base head's, weighted by the
    /// head's weight in each region, mixed by the joint's weight for the region.
    ///
    /// The result is in the order of the rig's joints, in metres, relative to the parent joint.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::MatrixShape`] for a matrix of another rig and the errors of
    /// [`DnaRig::variant_joints`] and [`DnaRig::joint_weights`].
    pub fn neutral_joint_offsets(&self, matrix: &DnaMatrix) -> Result<Vec<[f32; 3]>, RigError> {
        matrix.check(self)?;
        let joints = self.variant_joints()?;
        let weights = self.joint_weights()?;
        let mut offsets = Vec::with_capacity(joints.joints());
        for joint in 0..joints.joints() {
            let base = joints.translation(0, joint).unwrap_or_default();
            let mut offset = [0.0_f32; 3];
            for variant in 1..matrix.variants() {
                let fold: f32 = (0..matrix.regions())
                    .map(|region| {
                        weights.weight(joint, region).unwrap_or_default()
                            * matrix.cell(variant, region)
                    })
                    .sum();
                if fold == 0.0 {
                    continue;
                }
                let own = joints.translation(variant, joint).unwrap_or_default();
                for ((total, own), base) in offset.iter_mut().zip(own).zip(base) {
                    *total = fold.mul_add(own - base, *total);
                }
            }
            offsets.push(offset);
        }
        Ok(offsets)
    }
}
