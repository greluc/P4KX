// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_assembly::Armour;
use p4kx_extraction::model::{BoneMatch, Lod, Mesh, Model};
use rayon::prelude::*;

use super::skin::{UNWEIGHTED, of_mesh_matched};
use super::{
    Bounds, IDENTITY, Part, Rows, Scene, SceneError, Skeleton,
    check_size, next_serial, world_transforms,
};

/// Where the first mesh of a part of an Armour sits in the vertices of its [`Scene`]: the mesh a
/// morph target of the part moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MorphSpan {
    start: usize,
    len: usize,
}

impl MorphSpan {
    /// Returns the number of vertices of the mesh.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the mesh has no vertices.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

struct Piece<'a> {
    part: usize,
    mesh: &'a Mesh,
    world: &'a Rows,
    matched: &'a BoneMatch,
}

fn level_of(model: &Model, level: u32) -> Option<&Lod> {
    model
        .lods
        .iter()
        .filter(|lod| lod.level <= level)
        .max_by_key(|lod| lod.level)
        .or_else(|| model.lods.iter().min_by_key(|lod| lod.level))
}

impl Scene {
    /// Flattens the parts of `armour` at the level it was built at into one skinned triangle list
    /// on the skeleton of its body, each skin bound to the bones of the body by name like the GLB
    /// does. Returns the scene and, per part, the place of its first mesh.
    ///
    /// # Errors
    ///
    /// Returns [`SceneError::Empty`] without triangles, [`SceneError::TooLarge`] above
    /// [`MAX_SCENE_VERTICES`] or [`MAX_SCENE_TRIANGLES`], and [`SceneError::NotFinite`] when no
    /// vertex has finite coordinates.
    pub fn from_armour(armour: &Armour) -> Result<(Self, Vec<Option<MorphSpan>>), SceneError> {
        let bones = &armour.skeleton().bones;
        let skeleton = Skeleton::of(bones).map(Arc::new);
        let count = skeleton.as_ref().map_or(0, |one| one.bone_count());
        let worlds: Vec<Vec<Rows>> = armour
            .parts()
            .iter()
            .map(|part| world_transforms(&part.model().nodes))
            .collect();
        let matches: Vec<Option<BoneMatch>> = armour
            .parts()
            .iter()
            .map(|part| {
                level_of(part.model(), armour.lod())
                    .map(|lod| BoneMatch::new(bones, part.model().bones_of(lod)))
            })
            .collect();
        let mut pieces = Vec::new();
        let (mut vertices, mut triangles) = (0_usize, 0_usize);
        for (at, part) in armour.parts().iter().enumerate() {
            let (Some(lod), Some(Some(matched)), Some(world)) =
                (level_of(part.model(), armour.lod()), matches.get(at), worlds.get(at))
            else {
                continue;
            };
            for mesh in &lod.meshes {
                vertices = vertices.saturating_add(mesh.positions.len());
                triangles = triangles.saturating_add(mesh.indices.len() / 3);
                pieces.push(Piece {
                    part: at,
                    mesh,
                    world: world.get(mesh.node).unwrap_or(&IDENTITY),
                    matched,
                });
            }
        }
        check_size(vertices, triangles)?;
        let built: Vec<Part> = pieces
            .par_iter()
            .map(|piece| {
                let skin = of_mesh_matched(piece.mesh, count, piece.matched);
                Part::with_skin(piece.mesh, piece.world, skin)
            })
            .collect();
        let skinned = built.iter().any(|part| !part.skin.is_empty());
        let mut all_vertices = Vec::with_capacity(vertices);
        let mut all_indices = Vec::with_capacity(triangles.saturating_mul(3));
        let mut all_skin = Vec::with_capacity(if skinned { vertices } else { 0 });
        let mut spans = vec![None; armour.parts().len()];
        let mut bounds = Bounds::EMPTY;
        for (piece, part) in pieces.iter().zip(built) {
            let base = u32::try_from(all_vertices.len()).map_err(|_| SceneError::TooLarge {
                vertices,
                triangles,
            })?;
            if let Some(slot @ None) = spans.get_mut(piece.part) {
                *slot = Some(MorphSpan {
                    start: all_vertices.len(),
                    len: part.vertices.len(),
                });
            }
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
        let scene = Self {
            vertices: all_vertices,
            indices: all_indices,
            skin: skinned.then_some(all_skin),
            skeleton,
            bounds,
            serial: next_serial(),
        };
        Ok((scene, spans))
    }

    /// Returns the scene with the vertices `vertices` of the mesh at `span` moved by `weight`
    /// times `deltas`, one move per vertex; moves of vertices the mesh lacks are ignored. The
    /// scene has a serial of its own, so the GPU takes it as new.
    #[must_use]
    pub fn morphed(&self, span: MorphSpan, (vertices, deltas): (&[u32], &[[f32; 3]]), weight: f32) -> Self {
        let mut moved = self.vertices.clone();
        for (&vertex, delta) in vertices.iter().zip(deltas) {
            let Some(at) = usize::try_from(vertex)
                .ok()
                .filter(|&at| at < span.len)
                .map(|at| span.start + at)
            else {
                continue;
            };
            if let Some(place) = moved.get_mut(at) {
                for (axis, change) in delta.iter().enumerate() {
                    if let Some(value) = place.get_mut(axis) {
                        *value += change * weight;
                    }
                }
            }
        }
        Self {
            vertices: moved,
            indices: self.indices.clone(),
            skin: self.skin.clone(),
            skeleton: self.skeleton.clone(),
            bounds: self.bounds,
            serial: next_serial(),
        }
    }
}

