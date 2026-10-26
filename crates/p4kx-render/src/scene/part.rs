// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::model::Mesh;

use super::skin;
use super::{Bounds, IDENTITY, Rows, SkinVertex, Vertex, cell};
use crate::camera::{cross, dot};

/// One mesh flattened into model space.
pub(crate) struct Part {
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) indices: Vec<u32>,
    pub(crate) skin: Vec<SkinVertex>,
    pub(crate) bounds: Bounds,
}

impl Part {
    /// Flattens `mesh` bound to a skeleton of `bones` bones: a mesh with bone weights keeps its
    /// vertices in the space of the skeleton and ignores the transform of its node.
    pub(crate) fn of(mesh: &Mesh, world: &Rows, bones: usize) -> Self {
        Self::with_skin(mesh, world, skin::of_mesh(mesh, bones))
    }

    /// Flattens `mesh` with the given `skin`; a mesh with a skin ignores `world`.
    pub(crate) fn with_skin(mesh: &Mesh, world: &Rows, skin: Vec<SkinVertex>) -> Self {
        let world = if skin.is_empty() { world } else { &IDENTITY };
        Self::placed(mesh, world, skin)
    }

    /// Flattens `mesh` of a static model with its node transform `world` applied; `driver` is the
    /// bone that moves the whole mesh, or `None` for a mesh no bone moves.
    pub(crate) fn rigid(mesh: &Mesh, world: &Rows, driver: Option<usize>) -> Self {
        let skin = driver.map_or_else(Vec::new, |bone| skin::rigid(mesh, bone));
        Self::placed(mesh, world, skin)
    }

    fn placed(mesh: &Mesh, world: &Rows, skin: Vec<SkinVertex>) -> Self {
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
            skin,
            bounds,
        }
    }
}

pub(crate) fn apply(rows: &Rows, [x, y, z]: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| {
        cell(rows, row, 0).mul_add(
            x,
            cell(rows, row, 1).mul_add(y, cell(rows, row, 2).mul_add(z, cell(rows, row, 3))),
        )
    })
}

pub(super) fn rotate(rows: &[[f32; 3]; 3], [x, y, z]: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| {
        let values = rows.get(row).copied().unwrap_or([0.0; 3]);
        values[0].mul_add(x, values[1].mul_add(y, values[2] * z))
    })
}

pub(super) fn rows3(rows: &Rows) -> [[f32; 3]; 3] {
    std::array::from_fn(|row| std::array::from_fn(|column| cell(rows, row, column)))
}

pub(super) fn determinant(rows: &Rows) -> f32 {
    let [first, second, third] = rows3(rows);
    dot(first, cross(second, third))
}

pub(super) fn normal_matrix(rows: &Rows) -> [[f32; 3]; 3] {
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
