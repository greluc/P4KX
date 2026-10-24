// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_assembly::Assembly;
use p4kx_extraction::TextureLoader;
use p4kx_extraction::model::Model;
use p4kx_settings::ModelMaterials;
use rayon::prelude::*;
use thiserror::Error;

use super::flatten::{Range, flatten};
use super::surface::{Surface, SurfaceKey, surface_key, surfaces_of};
use super::{
    AssemblyScene, Group, Item, Left, MAX_VIEW_ITEMS, MAX_VIEW_TEXTURE_BYTES, MAX_VIEW_TRIANGLES,
    MAX_VIEW_VERTICES, PartDraw, PartInfo, VIEW_TEXTURE_MIP, ViewTexture,
};
use crate::scene::{Bounds, apply, next_serial};

const WAVE: usize = 64;
const TEXTURE_WAVE: usize = 16;
const MAX_TEXTURE_SIDE: u32 = 4096;

/// Why an Assembly cannot be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum BuildError {
    /// No part has triangles the viewer can show.
    #[error("the Assembly has no triangles to show")]
    Empty,
    /// Building was stopped.
    #[error("building the view was stopped")]
    Cancelled,
}

/// How far building the scene has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    /// Models flattened of the distinct ones.
    Flattening {
        /// How many are done.
        done: usize,
        /// How many there are.
        total: usize,
    },
    /// Textures read of the distinct ones.
    Textures {
        /// How many are done.
        done: usize,
        /// How many there are.
        total: usize,
    },
}

/// What the scene is drawn with.
#[derive(Clone, Copy)]
pub struct BuildOptions<'a> {
    /// Plain colours, or the diffuse textures over them.
    pub materials: ModelMaterials,
    /// Where the textures come from; without it the materials keep their plain colours.
    pub textures: Option<&'a dyn TextureLoader>,
}

/// The ways to stop building and to watch it.
#[derive(Clone, Copy)]
pub struct Hooks<'a> {
    /// Returns whether building should stop.
    pub cancelled: &'a (dyn Fn() -> bool + Sync),
    /// Called as building advances, from any thread.
    pub progress: &'a (dyn Fn(Progress) + Sync),
}

pub(super) struct SharedMesh {
    pub(super) ranges: Vec<Range>,
    bounds: Bounds,
    triangles: usize,
}

type GroupKey = (usize, usize, Option<u32>, bool);

fn address<T>(value: &T) -> usize {
    std::ptr::from_ref(value) as usize
}

impl AssemblyScene {
    /// Flattens `assembly`: each distinct model once, in parallel, then the parts as instances
    /// of those meshes with the surface each material range resolves to.
    ///
    /// # Errors
    ///
    /// Returns [`BuildError::Cancelled`] when `hooks` ask to stop and [`BuildError::Empty`] when
    /// nothing can be shown.
    pub fn build(
        assembly: &Assembly,
        options: &BuildOptions<'_>,
        hooks: &Hooks<'_>,
    ) -> Result<Self, BuildError> {
        let parts = assembly.parts();
        let mut ids: HashMap<usize, usize> = HashMap::new();
        let mut distinct: Vec<&Model> = Vec::new();
        let model_of: Vec<Option<usize>> = parts
            .iter()
            .map(|part| {
                part.model().map(|model| {
                    *ids.entry(address(model)).or_insert_with(|| {
                        distinct.push(model);
                        distinct.len() - 1
                    })
                })
            })
            .collect();
        let mut scene = Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            groups: Vec::new(),
            items: Vec::new(),
            draws: Vec::new(),
            parts: Vec::new(),
            textures: Vec::new(),
            meshes: 0,
            bounds: Bounds::EMPTY,
            left: Left::default(),
            serial: next_serial(),
        };
        let meshes = scene.merge_meshes(&distinct, assembly.lod(), hooks)?;
        scene.meshes = meshes.iter().flatten().count();
        let (surfaces, wanted) = surfaces_of(assembly, &model_of, &meshes, options.materials);
        let textures = scene.read_textures(&wanted, options, hooks)?;
        scene.place(assembly, (&model_of, &meshes), (&surfaces, &textures));
        if scene.draws.is_empty() || !scene.bounds.is_valid() {
            return Err(BuildError::Empty);
        }
        Ok(scene)
    }

    fn merge_meshes(
        &mut self,
        distinct: &[&Model],
        lod: u32,
        hooks: &Hooks<'_>,
    ) -> Result<Vec<Option<SharedMesh>>, BuildError> {
        let mut meshes = Vec::with_capacity(distinct.len());
        for wave in distinct.chunks(WAVE) {
            if (hooks.cancelled)() {
                return Err(BuildError::Cancelled);
            }
            let flats: Vec<_> = wave.par_iter().map(|model| flatten(model, lod)).collect();
            for flat in flats {
                let mesh = flat.and_then(|flat| {
                    let vertices = self.vertices.len().checked_add(flat.vertices.len())?;
                    let triangles = (self.indices.len() + flat.indices.len()) / 3;
                    if vertices > MAX_VIEW_VERTICES || triangles > MAX_VIEW_TRIANGLES {
                        return None;
                    }
                    let base = u32::try_from(self.vertices.len()).ok()?;
                    let first = u32::try_from(self.indices.len()).ok()?;
                    let count = flat.indices.len() / 3;
                    self.indices
                        .extend(flat.indices.iter().map(|index| index.saturating_add(base)));
                    self.vertices.extend(flat.vertices);
                    Some(SharedMesh {
                        ranges: flat
                            .ranges
                            .into_iter()
                            .map(|range| Range {
                                first_index: range.first_index + first,
                                ..range
                            })
                            .collect(),
                        bounds: flat.bounds,
                        triangles: count,
                    })
                });
                meshes.push(mesh);
            }
            (hooks.progress)(Progress::Flattening {
                done: meshes.len(),
                total: distinct.len(),
            });
        }
        Ok(meshes)
    }

    fn read_textures(
        &mut self,
        wanted: &[String],
        options: &BuildOptions<'_>,
        hooks: &Hooks<'_>,
    ) -> Result<HashMap<String, u32>, BuildError> {
        let mut kept = HashMap::new();
        let Some(loader) = options.textures.filter(|_| !wanted.is_empty()) else {
            self.left.textures = wanted.len();
            return Ok(kept);
        };
        let mut bytes = 0_usize;
        for (done, wave) in wanted.chunks(TEXTURE_WAVE).enumerate() {
            if (hooks.cancelled)() {
                return Err(BuildError::Cancelled);
            }
            let images: Vec<_> = wave
                .par_iter()
                .map(|path| loader.load(path, VIEW_TEXTURE_MIP).ok())
                .collect();
            for (path, texture) in wave.iter().zip(images) {
                let image = texture.map(|texture| texture.image).filter(|image| {
                    image.width() > 0
                        && image.height() > 0
                        && image.width() <= MAX_TEXTURE_SIDE
                        && image.height() <= MAX_TEXTURE_SIDE
                });
                let Some(image) = image else {
                    self.left.textures += 1;
                    continue;
                };
                let size = image.pixels().len();
                if bytes.saturating_add(size) > MAX_VIEW_TEXTURE_BYTES {
                    self.left.textures += 1;
                    continue;
                }
                bytes += size;
                kept.insert(
                    path.clone(),
                    u32::try_from(self.textures.len()).unwrap_or(0),
                );
                self.textures.push(ViewTexture::new(
                    image.width(),
                    image.height(),
                    image.into_pixels(),
                ));
            }
            (hooks.progress)(Progress::Textures {
                done: ((done + 1) * TEXTURE_WAVE).min(wanted.len()),
                total: wanted.len(),
            });
        }
        Ok(kept)
    }

    fn place(
        &mut self,
        assembly: &Assembly,
        (model_of, meshes): (&[Option<usize>], &[Option<SharedMesh>]),
        (surfaces, textures): (&HashMap<SurfaceKey, Option<Surface>>, &HashMap<String, u32>),
    ) {
        let mut groups: HashMap<GroupKey, u32> = HashMap::new();
        for (at, part) in assembly.parts().iter().enumerate() {
            let mesh = model_of
                .get(at)
                .copied()
                .flatten()
                .and_then(|id| meshes.get(id).map(|mesh| (id, mesh)))
                .and_then(|(id, mesh)| mesh.as_ref().map(|mesh| (id, mesh)));
            let mut info = PartInfo {
                name: part.name().to_owned(),
                path: part.path().to_owned(),
                parent: part.parent(),
                triangles: 0,
            };
            let Some((id, mesh)) = mesh else {
                if part.model().is_some() {
                    self.left.parts += 1;
                }
                self.parts.push(info);
                continue;
            };
            let first = self.items.len();
            let pending = self.items_of(part, (id, mesh), (surfaces, textures), &mut groups);
            if pending.is_empty() {
                self.parts.push(info);
                continue;
            }
            if first + pending.len() > MAX_VIEW_ITEMS {
                self.left.parts += 1;
                self.parts.push(info);
                continue;
            }
            let Some((centre, radius)) = self.include(&mesh.bounds, &part.world().rows) else {
                self.left.parts += 1;
                self.parts.push(info);
                continue;
            };
            info.triangles = mesh.triangles;
            self.items.extend(pending);
            self.draws.push(PartDraw {
                part: at,
                world: part.world().rows,
                centre,
                radius,
                items: u32::try_from(first).unwrap_or(u32::MAX)
                    ..u32::try_from(self.items.len()).unwrap_or(u32::MAX),
            });
            self.parts.push(info);
        }
    }

    fn items_of(
        &mut self,
        part: &p4kx_assembly::Part,
        (id, mesh): (usize, &SharedMesh),
        (surfaces, textures): (&HashMap<SurfaceKey, Option<Surface>>, &HashMap<String, u32>),
        groups: &mut HashMap<GroupKey, u32>,
    ) -> Vec<Item> {
        let mut pending = Vec::new();
        for (position, range) in mesh.ranges.iter().enumerate() {
            let key = surface_key(part, range.material);
            let Some(surface) = surfaces.get(&key).and_then(Option::as_ref) else {
                continue;
            };
            let texture = surface
                .texture
                .as_ref()
                .and_then(|path| textures.get(path))
                .copied();
            let next = u32::try_from(self.groups.len()).unwrap_or(u32::MAX);
            let group = *groups
                .entry((id, position, texture, surface.translucent))
                .or_insert(next);
            if group == next {
                self.groups.push(Group {
                    first_index: range.first_index,
                    index_count: range.index_count,
                    texture,
                    translucent: surface.translucent,
                });
            }
            pending.push(Item {
                group,
                colour: surface.colour,
            });
        }
        pending
    }

    fn include(&mut self, local: &Bounds, world: &[[f32; 4]; 3]) -> Option<([f32; 3], f32)> {
        let mut placed = Bounds::EMPTY;
        for corner in 0..8_usize {
            let pick = |axis: usize| {
                let side = if corner >> axis & 1 == 1 {
                    &local.max
                } else {
                    &local.min
                };
                side.get(axis).copied().unwrap_or(0.0)
            };
            placed.include(apply(world, [pick(0), pick(1), pick(2)]));
        }
        placed.is_valid().then(|| {
            self.bounds.union(&placed);
            (placed.centre(), placed.radius())
        })
    }
}
