// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use p4kx_extraction::model::{Lod, Model, Node, Transform};
use rayon::prelude::*;
use thiserror::Error;

pub use super::bounds::Bounds;
use super::skeleton::Skeleton;
pub(crate) use part::{Part, apply};
use part::{rotate, rows3};
use skin::UNWEIGHTED;

/// The most vertices a [`Scene`] holds; a model with more is refused.
pub const MAX_SCENE_VERTICES: usize = 4_000_000;

/// The most triangles a [`Scene`] holds; a model with more is refused.
pub const MAX_SCENE_TRIANGLES: usize = 8_000_000;

/// One vertex as the GPU reads it: position, normal and texture coordinate.
pub type Vertex = [f32; 8];

/// The bytes of the skin of one vertex: four bone indices and four weights.
pub const SKIN_STRIDE: usize = 12;

/// The four bones that move a vertex and their weights.
pub type SkinVertex = [u8; SKIN_STRIDE];

static SERIAL: AtomicU64 = AtomicU64::new(1);

/// Returns a number no scene of this process has had; the GPU caches are keyed by it.
pub(super) fn next_serial() -> u64 {
    SERIAL.fetch_add(1, Ordering::Relaxed)
}

pub(super) type Rows = [[f32; 4]; 3];

pub(super) const IDENTITY: Rows = Transform::IDENTITY.rows;

/// Why a model cannot be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SceneError {
    /// The model has no triangles at its most detailed level.
    #[error("the model has no triangles")]
    Empty,
    /// The model has more vertices or triangles than the viewer takes.
    #[error("the model has {vertices} vertices and {triangles} triangles")]
    TooLarge {
        /// The number of vertices of the most detailed level.
        vertices: usize,
        /// The number of triangles of the most detailed level.
        triangles: usize,
    },
    /// No vertex of the model has finite coordinates.
    #[error("the model has no finite coordinates")]
    NotFinite,
}

/// The most detailed level of a model flattened into one triangle list in model space: Z up,
/// metres, every node transform applied. The viewer draws exactly this.
#[derive(Debug)]
pub struct Scene {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    skin: Option<Vec<SkinVertex>>,
    skeleton: Option<Arc<Skeleton>>,
    bounds: Bounds,
    serial: u64,
}

impl Scene {
    /// Flattens the most detailed level of `model`.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::Empty`] without triangles, [`SceneError::TooLarge`] above
    /// [`MAX_SCENE_VERTICES`] or [`MAX_SCENE_TRIANGLES`], and [`SceneError::NotFinite`] when no
    /// vertex has finite coordinates.
    pub fn from_model(model: &Model) -> Result<Self, SceneError> {
        let lod = model
            .lods
            .iter()
            .find(|lod| lod.level == 0)
            .ok_or(SceneError::Empty)?;
        let (vertices, triangles) = (lod.vertex_count(), lod.triangle_count());
        check_size(vertices, triangles)?;
        let (skeleton, parts) = parts_of(model, lod);
        let skinned = parts.iter().any(|part| !part.skin.is_empty());
        let mut all_vertices = Vec::with_capacity(vertices);
        let mut all_indices = Vec::with_capacity(triangles.saturating_mul(3));
        let mut all_skin = Vec::with_capacity(if skinned { vertices } else { 0 });
        let mut bounds = Bounds::EMPTY;
        for part in parts {
            let base = u32::try_from(all_vertices.len()).map_err(|_| SceneError::TooLarge {
                vertices,
                triangles,
            })?;
            bounds.union(&part.bounds);
            all_indices.extend(part.indices.iter().map(|index| index.saturating_add(base)));
            if skinned {
                if part.skin.is_empty() {
                    all_skin.extend(std::iter::repeat_n(UNWEIGHTED, part.vertices.len()));
                } else {
                    all_skin.extend(part.skin);
                }
            }
            all_vertices.extend(part.vertices);
        }
        if !bounds.is_valid() {
            return Err(SceneError::NotFinite);
        }
        Ok(Self {
            vertices: all_vertices,
            indices: all_indices,
            skin: skinned.then_some(all_skin),
            skeleton,
            bounds,
            serial: next_serial(),
        })
    }

    /// Returns the vertices.
    #[must_use]
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Returns the triangle list: three vertex indices per triangle.
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Returns, per vertex, the four bones that move it and their weights: four little-endian
    /// `u16` bone indices and four `u8` weights, 12 bytes; `None` for a model without skinned meshes
    /// or without a usable skeleton, and for a static model none of whose helper bones drives a
    /// node of a mesh. In a static model the meshes under a node a helper bone drives are bound
    /// wholly to that bone. A vertex with all weights zero stays where it is.
    #[must_use]
    pub fn skin(&self) -> Option<&[[u8; SKIN_STRIDE]]> {
        self.skin.as_deref()
    }

    /// Returns the skeleton of the most detailed level, or `None` for a model without one.
    #[must_use]
    pub const fn skeleton(&self) -> Option<&Arc<Skeleton>> {
        self.skeleton.as_ref()
    }

    /// Returns the box around the finite vertices.
    #[must_use]
    pub const fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    /// Returns the number of triangles.
    #[must_use]
    pub const fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Returns a number no other scene of this process has; the GPU caches are keyed by it.
    #[must_use]
    pub const fn serial(&self) -> u64 {
        self.serial
    }
}

/// Flattens every mesh of `lod` and returns them with the skeleton they are bound to. A mesh with
/// bone weights is bound by its weights; in a model without any, the meshes are bound rigidly to
/// the helper bones that drive their nodes.
fn parts_of(model: &Model, lod: &Lod) -> (Option<Arc<Skeleton>>, Vec<Part>) {
    let world = world_transforms(&model.nodes);
    let skeleton = Skeleton::of(model.bones_of(lod)).map(Arc::new);
    let bones = skeleton
        .as_ref()
        .map_or(0, |skeleton| skeleton.bone_count());
    let drivers = skeleton
        .as_ref()
        .filter(|_| lod.meshes.iter().all(|mesh| mesh.influences.is_none()))
        .map(|_| skin::drivers(&model.nodes, model.bones_of(lod)));
    let parts = lod
        .meshes
        .par_iter()
        .map(|mesh| {
            let placed = world.get(mesh.node).unwrap_or(&IDENTITY);
            drivers.as_ref().map_or_else(
                || Part::of(mesh, placed, bones),
                |drivers| Part::rigid(mesh, placed, drivers.get(mesh.node).copied().flatten()),
            )
        })
        .collect();
    (skeleton, parts)
}

/// Checks the counts of a level of detail against the limits of a scene.
const fn check_size(vertices: usize, triangles: usize) -> Result<(), SceneError> {
    if vertices == 0 || triangles == 0 {
        Err(SceneError::Empty)
    } else if vertices > MAX_SCENE_VERTICES || triangles > MAX_SCENE_TRIANGLES {
        Err(SceneError::TooLarge {
            vertices,
            triangles,
        })
    } else {
        Ok(())
    }
}

pub(super) fn world_transforms(nodes: &[Node]) -> Vec<Rows> {
    let mut done: Vec<Option<Rows>> = vec![None; nodes.len()];
    for start in 0..nodes.len() {
        let mut chain = Vec::new();
        let mut at = Some(start);
        let mut base = IDENTITY;
        while let Some(index) = at {
            if let Some(Some(known)) = done.get(index) {
                base = *known;
                break;
            }
            if chain.len() > nodes.len() {
                break;
            }
            chain.push(index);
            at = nodes.get(index).and_then(|node| node.parent);
        }
        for &index in chain.iter().rev() {
            if let Some(node) = nodes.get(index) {
                base = multiply(&base, &node.transform.rows);
                if let Some(slot) = done.get_mut(index) {
                    *slot = Some(base);
                }
            }
        }
    }
    done.into_iter()
        .map(|rows| rows.unwrap_or(IDENTITY))
        .collect()
}

fn multiply(outer: &Rows, inner: &Rows) -> Rows {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            let linear: f32 = (0..3)
                .map(|k| cell(outer, row, k) * cell(inner, k, column))
                .sum();
            if column == 3 {
                linear + cell(outer, row, 3)
            } else {
                linear
            }
        })
    })
}

fn cell(rows: &Rows, row: usize, column: usize) -> f32 {
    rows.get(row)
        .and_then(|values| values.get(column))
        .copied()
        .unwrap_or(0.0)
}

mod armour;
mod part;
mod skin;
#[cfg(test)]
mod tests;

pub use armour::MorphSpan;
