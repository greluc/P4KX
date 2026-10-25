// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_assembly::Armour;
use p4kx_extraction::model::{BoneMatch, Lod, Mesh, Model};
use rayon::prelude::*;

use super::skin::{UNWEIGHTED, of_mesh_matched};
use super::{
    Bounds, IDENTITY, Part, Rows, Scene, SceneError, Skeleton, check_size, next_serial, rotate,
    rows3, world_transforms,
};

/// Where the first mesh of a part of an Armour sits in the vertices of its [`Scene`]: the mesh a
/// morph target of the part moves, and the turn that takes a delta of the mesh to where its
/// vertices sit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MorphSpan {
    start: usize,
    len: usize,
    linear: [[f32; 3]; 3],
}

impl MorphSpan {
    const IDENTITY: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    fn placed(start: usize, len: usize, world: &Rows, skinned: bool) -> Self {
        Self {
            start,
            len,
            linear: if skinned {
                Self::IDENTITY
            } else {
                rows3(world)
            },
        }
    }

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
    fit: &'a BoneMatch,
}

struct Gathered<'a> {
    pieces: Vec<Piece<'a>>,
    vertices: usize,
    triangles: usize,
}

fn level_of(model: &Model, level: u32) -> Option<&Lod> {
    model
        .lods
        .iter()
        .filter(|lod| lod.level <= level)
        .max_by_key(|lod| lod.level)
        .or_else(|| model.lods.iter().min_by_key(|lod| lod.level))
}

fn gather<'a>(
    armour: &'a Armour,
    worlds: &'a [Vec<Rows>],
    fits: &'a [Option<BoneMatch>],
) -> Gathered<'a> {
    let mut gathered = Gathered {
        pieces: Vec::new(),
        vertices: 0,
        triangles: 0,
    };
    for (at, part) in armour.parts().iter().enumerate() {
        let (Some(lod), Some(Some(fit)), Some(world)) = (
            level_of(part.model(), armour.lod()),
            fits.get(at),
            worlds.get(at),
        ) else {
            continue;
        };
        for mesh in &lod.meshes {
            gathered.vertices = gathered.vertices.saturating_add(mesh.positions.len());
            gathered.triangles = gathered.triangles.saturating_add(mesh.indices.len() / 3);
            gathered.pieces.push(Piece {
                part: at,
                mesh,
                world: world.get(mesh.node).unwrap_or(&IDENTITY),
                fit,
            });
        }
    }
    gathered
}

fn joined(
    gathered: &Gathered<'_>,
    built: Vec<Part>,
    (parts, skeleton): (usize, Option<Arc<Skeleton>>),
) -> Result<(Scene, Vec<Option<MorphSpan>>), SceneError> {
    let (vertices, triangles) = (gathered.vertices, gathered.triangles);
    let skinned = built.iter().any(|part| !part.skin.is_empty());
    let mut all_vertices = Vec::with_capacity(vertices);
    let mut all_indices = Vec::with_capacity(triangles.saturating_mul(3));
    let mut all_skin = Vec::with_capacity(if skinned { vertices } else { 0 });
    let mut spans = vec![None; parts];
    let mut bounds = Bounds::EMPTY;
    for (piece, part) in gathered.pieces.iter().zip(built) {
        let base = u32::try_from(all_vertices.len()).map_err(|_| SceneError::TooLarge {
            vertices,
            triangles,
        })?;
        if let Some(slot @ None) = spans.get_mut(piece.part) {
            *slot = Some(MorphSpan::placed(
                all_vertices.len(),
                part.vertices.len(),
                piece.world,
                !part.skin.is_empty(),
            ));
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
    let scene = Scene {
        vertices: all_vertices,
        indices: all_indices,
        skin: skinned.then_some(all_skin),
        skeleton,
        bounds,
        serial: next_serial(),
    };
    Ok((scene, spans))
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
        let fits: Vec<Option<BoneMatch>> = armour
            .parts()
            .iter()
            .map(|part| {
                level_of(part.model(), armour.lod())
                    .map(|lod| BoneMatch::new(bones, part.model().bones_of(lod)))
            })
            .collect();
        let gathered = gather(armour, &worlds, &fits);
        check_size(gathered.vertices, gathered.triangles)?;
        let built: Vec<Part> = gathered
            .pieces
            .par_iter()
            .map(|piece| {
                let skin = of_mesh_matched(piece.mesh, count, piece.fit);
                Part::with_skin(piece.mesh, piece.world, skin)
            })
            .collect();
        joined(&gathered, built, (armour.parts().len(), skeleton))
    }

    /// Returns the scene with the vertices `vertices` of the mesh at `span` moved by `weight`
    /// times `deltas`, one move per vertex, turned like the mesh when it is not skinned; moves of
    /// vertices the mesh lacks are ignored. The scene has a serial of its own, so the GPU takes it
    /// as new.
    #[must_use]
    pub fn morphed(
        &self,
        span: MorphSpan,
        (vertices, deltas): (&[u32], &[[f32; 3]]),
        weight: f32,
    ) -> Self {
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
                for (axis, change) in rotate(&span.linear, *delta).iter().enumerate() {
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

#[cfg(test)]
mod tests {
    use super::{Bounds, MorphSpan, Rows, Scene, next_serial};

    const QUARTER_TURN: Rows = [
        [0.0, -1.0, 0.0, 5.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ];

    fn scene() -> Scene {
        Scene {
            vertices: vec![[0.0; 8]; 2],
            indices: vec![0, 1, 1],
            skin: None,
            skeleton: None,
            bounds: Bounds::EMPTY,
            serial: next_serial(),
        }
    }

    const MIRRORED_STRETCH: Rows = [
        [-2.0, 0.0, 0.0, 7.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 3.0, 0.0],
    ];

    fn moved_by(world: &Rows, skinned: bool, delta: [f32; 3]) -> [f32; 3] {
        let span = MorphSpan::placed(1, 1, world, skinned);
        let after = scene().morphed(span, (&[0], &[delta]), 2.0);
        let [x, y, z, ..] = after.vertices()[1];
        [x, y, z]
    }

    fn moved(skinned: bool) -> [f32; 3] {
        moved_by(&QUARTER_TURN, skinned, [1.0, 0.0, 0.0])
    }

    #[test]
    fn a_delta_of_a_mirrored_and_stretched_mesh_follows_its_vertices() {
        assert_eq!(
            moved_by(&MIRRORED_STRETCH, false, [1.0, 1.0, 1.0]),
            [-4.0, 2.0, 6.0]
        );
    }

    #[test]
    fn a_delta_of_an_unskinned_mesh_is_turned_like_the_mesh_but_not_moved_with_it() {
        assert_eq!(moved(false), [0.0, 2.0, 0.0]);
    }

    #[test]
    fn a_delta_of_a_skinned_mesh_is_taken_as_it_is() {
        assert_eq!(moved(true), [2.0, 0.0, 0.0]);
    }
}
