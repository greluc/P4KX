// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::model::{Lod, Model};

use super::super::scene::{Bounds, IDENTITY, Part, Vertex, world_transforms};
use super::super::skeleton::Skeleton;

/// A range of a flattened model drawn with one material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Range {
    pub(super) first_index: u32,
    pub(super) index_count: u32,
    pub(super) material: u16,
}

/// A model at one level of detail as one triangle list in model space, node transforms applied.
#[derive(Debug)]
pub(super) struct Flat {
    pub(super) vertices: Vec<Vertex>,
    pub(super) indices: Vec<u32>,
    pub(super) ranges: Vec<Range>,
    pub(super) bounds: Bounds,
}

/// Picks the level of `model` nearest below `wanted`, or its most detailed one.
fn level_of(model: &Model, wanted: u32) -> Option<&Lod> {
    model
        .lods
        .iter()
        .filter(|lod| lod.level <= wanted)
        .max_by_key(|lod| lod.level)
        .or_else(|| model.lods.iter().min_by_key(|lod| lod.level))
}

/// Flattens the level of `model` that `wanted` selects, or returns `None` for a model without
/// triangles or with more than `u32` indices.
pub(super) fn flatten(model: &Model, wanted: u32) -> Option<Flat> {
    let lod = level_of(model, wanted)?;
    let world = world_transforms(&model.nodes);
    let skeleton = Skeleton::of(model.bones_of(lod));
    let bones = skeleton.as_ref().map_or(0, Skeleton::bone_count);
    let mut flat = Flat {
        vertices: Vec::with_capacity(lod.vertex_count()),
        indices: Vec::with_capacity(lod.triangle_count().saturating_mul(3)),
        ranges: Vec::new(),
        bounds: Bounds::EMPTY,
    };
    for mesh in &lod.meshes {
        let part = Part::of(mesh, world.get(mesh.node).unwrap_or(&IDENTITY), bones);
        let base = u32::try_from(flat.vertices.len()).ok()?;
        let first = u32::try_from(flat.indices.len()).ok()?;
        let count = u32::try_from(part.indices.len()).ok()?;
        flat.bounds.union(&part.bounds);
        flat.indices
            .extend(part.indices.iter().map(|index| index.saturating_add(base)));
        flat.vertices.extend(part.vertices);
        if mesh.submeshes.is_empty() {
            flat.ranges.push(Range {
                first_index: first,
                index_count: count - count % 3,
                material: 0,
            });
        }
        for submesh in &mesh.submeshes {
            let start = u32::try_from(submesh.first_index).ok()?;
            let length = u32::try_from(submesh.index_count).ok()?;
            let end = start.saturating_add(length).min(count);
            let length = end.saturating_sub(start);
            if length >= 3 {
                flat.ranges.push(Range {
                    first_index: first.checked_add(start)?,
                    index_count: length - length % 3,
                    material: submesh.material_id,
                });
            }
        }
    }
    (!flat.indices.is_empty() && flat.bounds.is_valid()).then_some(flat)
}
