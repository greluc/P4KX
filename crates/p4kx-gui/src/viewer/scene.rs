use std::sync::atomic::{AtomicU64, Ordering};

use p4kx_extraction::model::{Mesh, Model, Node, Transform};
use rayon::prelude::*;
use thiserror::Error;

use super::camera::{cross, dot};

/// The most vertices a [`Scene`] holds; a model with more is refused.
pub const MAX_SCENE_VERTICES: usize = 4_000_000;

/// The most triangles a [`Scene`] holds; a model with more is refused.
pub const MAX_SCENE_TRIANGLES: usize = 8_000_000;

/// One vertex as the GPU reads it: position, normal and texture coordinate.
pub type Vertex = [f32; 8];

static SERIAL: AtomicU64 = AtomicU64::new(1);

type Rows = [[f32; 4]; 3];

const IDENTITY: Rows = Transform::IDENTITY.rows;

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

/// An axis-aligned box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// The corner with the smallest coordinates.
    pub min: [f32; 3],
    /// The corner with the largest coordinates.
    pub max: [f32; 3],
}

impl Bounds {
    const EMPTY: Self = Self {
        min: [f32::INFINITY; 3],
        max: [f32::NEG_INFINITY; 3],
    };

    fn include(&mut self, point: [f32; 3]) {
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

    fn union(&mut self, other: &Self) {
        self.include(other.min);
        self.include(other.max);
    }

    fn is_valid(&self) -> bool {
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

/// The most detailed level of a model flattened into one triangle list in model space: Z up,
/// metres, every node transform applied. The viewer draws exactly this.
#[derive(Debug)]
pub struct Scene {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
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
        let world = world_transforms(&model.nodes);
        let parts: Vec<Part> = lod
            .meshes
            .par_iter()
            .map(|mesh| Part::of(mesh, world.get(mesh.node).unwrap_or(&IDENTITY)))
            .collect();
        let mut all_vertices = Vec::with_capacity(vertices);
        let mut all_indices = Vec::with_capacity(triangles.saturating_mul(3));
        let mut bounds = Bounds::EMPTY;
        for part in parts {
            let base = u32::try_from(all_vertices.len()).map_err(|_| SceneError::TooLarge {
                vertices,
                triangles,
            })?;
            bounds.union(&part.bounds);
            all_indices.extend(part.indices.iter().map(|index| index.saturating_add(base)));
            all_vertices.extend(part.vertices);
        }
        if !bounds.is_valid() {
            return Err(SceneError::NotFinite);
        }
        Ok(Self {
            vertices: all_vertices,
            indices: all_indices,
            bounds,
            serial: SERIAL.fetch_add(1, Ordering::Relaxed),
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

struct Part {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    bounds: Bounds,
}

impl Part {
    fn of(mesh: &Mesh, world: &Rows) -> Self {
        let normal_rows = normal_matrix(world);
        let mut bounds = Bounds::EMPTY;
        let vertices = mesh
            .positions
            .iter()
            .enumerate()
            .map(|(at, &position)| {
                let placed = apply(world, position);
                bounds.include(placed);
                let normal = mesh
                    .normals
                    .as_ref()
                    .and_then(|normals| normals.get(at))
                    .map_or([0.0; 3], |&normal| normalised(rotate(&normal_rows, normal)));
                let uv = mesh.uvs.get(at).copied().unwrap_or([0.0; 2]);
                [
                    placed[0], placed[1], placed[2], normal[0], normal[1], normal[2], uv[0], uv[1],
                ]
            })
            .collect();
        let mut indices = mesh.indices.clone();
        if determinant(world) < 0.0 {
            for triangle in indices.as_chunks_mut::<3>().0 {
                triangle.swap(1, 2);
            }
        }
        Self {
            vertices,
            indices,
            bounds,
        }
    }
}

fn world_transforms(nodes: &[Node]) -> Vec<Rows> {
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

fn apply(rows: &Rows, [x, y, z]: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| {
        cell(rows, row, 0).mul_add(
            x,
            cell(rows, row, 1).mul_add(y, cell(rows, row, 2).mul_add(z, cell(rows, row, 3))),
        )
    })
}

fn rotate(rows: &[[f32; 3]; 3], [x, y, z]: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| {
        let values = rows.get(row).copied().unwrap_or([0.0; 3]);
        values[0].mul_add(x, values[1].mul_add(y, values[2] * z))
    })
}

fn rows3(rows: &Rows) -> [[f32; 3]; 3] {
    std::array::from_fn(|row| std::array::from_fn(|column| cell(rows, row, column)))
}

fn determinant(rows: &Rows) -> f32 {
    let [first, second, third] = rows3(rows);
    dot(first, cross(second, third))
}

fn normal_matrix(rows: &Rows) -> [[f32; 3]; 3] {
    let [first, second, third] = rows3(rows);
    let det = determinant(rows);
    let scale = if det.abs() > f32::EPSILON {
        1.0 / det
    } else {
        1.0
    };
    [
        cross(second, third),
        cross(third, first),
        cross(first, second),
    ]
    .map(|row| row.map(|value| value * scale))
}

fn normalised(vector: [f32; 3]) -> [f32; 3] {
    let length = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if length.is_finite() && length > 0.0 {
        vector.map(|value| value / length)
    } else {
        [0.0; 3]
    }
}

#[cfg(test)]
mod tests;
