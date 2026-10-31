// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{DnaMatrix, DnaRig, ExpressionMorphs, RigError};

/// The deltas smaller than this on every axis, 0.1 micrometre, are dropped from a target.
pub(crate) const NEGLIGIBLE: f32 = 1e-7;

/// The expression blendshapes of a Character's head: the blendshapes of the stock heads of the rig
/// mixed by the Character's DNA.
///
/// Per vertex the heads are weighted like the neutral face is: the weight of a head at a vertex is
/// the weights the DNA gives it in the regions, summed over the regions by the vertex's region
/// mask, and what the heads leave of 1 belongs to the base head. A blendshape then is the sum of
/// the heads' blendshapes at their weights, so it moves the Character's own neutral face. Built by
/// [`DnaRig::expression_blend`].
#[derive(Debug, Clone)]
pub struct ExpressionBlend<'a> {
    morphs: ExpressionMorphs<'a>,
    vertices: usize,
    heads: Vec<(usize, Vec<f32>)>,
    base: Vec<f32>,
}

/// One blendshape of a Character's head as moves of vertices of the rig's mesh.
///
/// The vertices ascend; `deltas` holds the move in metres of each, in the rig's axes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExpressionTarget {
    /// The vertices the blendshape moves, ascending.
    pub vertices: Vec<u32>,
    /// The move of each vertex in metres.
    pub deltas: Vec<[f32; 3]>,
}

impl<'a> DnaRig<'a> {
    /// Prepares the expression blendshapes of the Character `matrix` stands for, see
    /// [`ExpressionBlend`].
    ///
    /// # Errors
    ///
    /// Returns [`RigError::MatrixShape`] for a matrix of another rig, the errors of
    /// [`DnaRig::expression_morphs`], [`DnaRig::covered_regions`] and [`DnaRig::region_masks`], and
    /// [`RigError::VertexCount`] when the region masks and the expression morphs disagree.
    pub fn expression_blend(&self, matrix: &DnaMatrix) -> Result<ExpressionBlend<'a>, RigError> {
        matrix.check(self)?;
        let morphs = self.expression_morphs()?;
        let covered = self.covered_regions()?;
        let masks = self.region_masks()?;
        let vertices = masks.vertices();
        let mut base = vec![1.0_f32; vertices];
        let mut heads = Vec::new();
        for variant in 1..matrix.variants() {
            let regions: Vec<(usize, f32)> = covered
                .iter()
                .enumerate()
                .map(|(local, &region)| (local, matrix.cell(variant, region)))
                .filter(|&(_, weight)| weight > 0.0)
                .collect();
            if regions.is_empty() {
                continue;
            }
            let weights: Vec<f32> = (0..vertices)
                .map(|vertex| {
                    regions
                        .iter()
                        .map(|&(local, weight)| {
                            weight * masks.weight(local, vertex).unwrap_or_default()
                        })
                        .sum()
                })
                .collect();
            for (rest, weight) in base.iter_mut().zip(&weights) {
                *rest -= weight;
            }
            heads.push((variant, weights));
        }
        for rest in &mut base {
            *rest = rest.max(0.0);
        }
        Ok(ExpressionBlend {
            morphs,
            vertices,
            heads,
            base,
        })
    }
}

impl ExpressionBlend<'_> {
    /// Returns the number of blendshapes.
    #[must_use]
    pub const fn blendshapes(&self) -> usize {
        self.morphs.blendshapes()
    }

    /// Returns the number of vertices of the mesh the blendshapes move.
    #[must_use]
    pub const fn vertices(&self) -> usize {
        self.vertices
    }

    /// Returns blendshape `blendshape` of the Character's head, without the moves under a tenth of
    /// a micrometre; empty for a blendshape no head moves or one outside the rig.
    #[must_use]
    pub fn target(&self, blendshape: usize) -> ExpressionTarget {
        let mut dense = vec![[0.0_f32; 3]; self.vertices];
        if let Some(cell) = self.morphs.cell(0, blendshape) {
            for (vertex, delta) in cell.moves() {
                add(&mut dense, vertex, weight_at(&self.base, vertex), delta);
            }
        }
        for (variant, weights) in &self.heads {
            let Some(cell) = self.morphs.cell(*variant, blendshape) else {
                continue;
            };
            for (vertex, delta) in cell.moves() {
                add(&mut dense, vertex, weight_at(weights, vertex), delta);
            }
        }
        let mut target = ExpressionTarget::default();
        for (vertex, delta) in dense.into_iter().enumerate() {
            if delta.iter().all(|value| value.is_finite())
                && delta.iter().any(|value| value.abs() >= NEGLIGIBLE)
                && let Ok(vertex) = u32::try_from(vertex)
            {
                target.vertices.push(vertex);
                target.deltas.push(delta);
            }
        }
        target
    }
}

fn weight_at(weights: &[f32], vertex: u32) -> f32 {
    usize::try_from(vertex)
        .ok()
        .and_then(|at| weights.get(at))
        .copied()
        .unwrap_or_default()
}

fn add(dense: &mut [[f32; 3]], vertex: u32, weight: f32, delta: [f32; 3]) {
    let Some(slot) = usize::try_from(vertex)
        .ok()
        .and_then(|at| dense.get_mut(at))
    else {
        return;
    };
    for (total, value) in slot.iter_mut().zip(delta) {
        *total = weight.mul_add(value, *total);
    }
}
