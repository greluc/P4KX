// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{BTreeMap, BTreeSet};

use gltf_json as json;
use json::Index;
use json::material::{
    AlphaCutoff, AlphaMode, EmissiveFactor, PbrBaseColorFactor, PbrMetallicRoughness,
    StrengthFactor,
};
use json::validation::Checked::Valid;
use p4kx_model::{Lod, Material, MaterialLibrary, TintPalette};

use crate::shading::shading;

const GLASS_ALPHA: f32 = 0.3;

const DECAL_SHADERS: [&str; 2] = ["MeshDecal", "Decal"];
const GLASS_SHADERS: [&str; 1] = ["GlassPBR"];

/// Which glTF material a material ID is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Slot {
    /// The sub-material at this position of the library.
    Library(usize),
    /// The engine's default material.
    Default,
}

/// How the material IDs of one level of detail map to materials, and which are not drawn.
pub(crate) struct MaterialPlan<'m> {
    library: Option<&'m MaterialLibrary>,
    slots: BTreeMap<u16, Option<Slot>>,
    default: Material,
    tinted: BTreeMap<usize, Material>,
}

impl<'m> MaterialPlan<'m> {
    /// Resolves every material ID `lod` uses against `library`, `None` when there is none.
    pub(crate) fn new(library: Option<&'m MaterialLibrary>, lod: &Lod) -> Self {
        let ids: BTreeSet<u16> = lod
            .meshes
            .iter()
            .flat_map(|mesh| &mesh.submeshes)
            .map(|submesh| submesh.material_id)
            .collect();
        let slots = ids
            .into_iter()
            .map(|id| {
                (
                    id,
                    library.map_or(Some(Slot::Default), |library| resolve(library, id)),
                )
            })
            .collect();
        Self {
            library,
            slots,
            default: Material::engine_default(),
            tinted: BTreeMap::new(),
        }
    }

    /// Draws the materials that take a palette colour with `palette`, if there is one.
    pub(crate) fn tinted_by(mut self, palette: Option<&TintPalette>) -> Self {
        let (Some(palette), Some(library)) = (palette, self.library) else {
            return self;
        };
        for slot in self.slots.values().flatten() {
            if let Slot::Library(position) = *slot
                && let Some(tinted) = library
                    .materials
                    .get(position)
                    .and_then(|material| palette.apply(material))
            {
                self.tinted.insert(position, tinted);
            }
        }
        self
    }

    /// Returns a plan for a model that has no level of detail to draw.
    pub(crate) fn empty() -> Self {
        Self {
            library: None,
            slots: BTreeMap::new(),
            default: Material::engine_default(),
            tinted: BTreeMap::new(),
        }
    }

    /// Returns whether nothing is drawn with `id`.
    pub(crate) fn hidden(&self, id: u16) -> bool {
        self.slots.get(&id).is_some_and(Option::is_none)
    }

    /// Returns the slot of `id`, or `None` when nothing is drawn with it.
    pub(crate) fn slot(&self, id: u16) -> Option<Slot> {
        self.slots.get(&id).copied().flatten()
    }

    /// Returns the distinct slots something is drawn with, in the order of the material IDs.
    pub(crate) fn drawn_slots(&self) -> Vec<Slot> {
        let mut drawn: Vec<Slot> = Vec::new();
        for slot in self.slots.values().flatten() {
            if !drawn.contains(slot) {
                drawn.push(*slot);
            }
        }
        drawn
    }

    /// Returns the number of material IDs drawn with the engine's default material.
    pub(crate) fn defaulted(&self) -> usize {
        self.slots
            .values()
            .filter(|slot| **slot == Some(Slot::Default))
            .count()
    }

    /// Returns the material of `slot`.
    pub(crate) fn material(&self, slot: Slot) -> &Material {
        match slot {
            Slot::Library(position) => self
                .tinted
                .get(&position)
                .or_else(|| {
                    self.library
                        .and_then(|library| library.materials.get(position))
                })
                .unwrap_or(&self.default),
            Slot::Default => &self.default,
        }
    }
}

fn resolve(library: &MaterialLibrary, id: u16) -> Option<Slot> {
    let position = if library.single { 0 } else { usize::from(id) };
    match library.materials.get(position) {
        Some(material) if material.hidden() => None,
        Some(_) => Some(Slot::Library(position)),
        None => Some(Slot::Default),
    }
}

/// The glTF textures a material is drawn with, besides its plain colours.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MaterialTextures {
    /// The base colour texture and whether the material's colours are already in it; if not, the
    /// diffuse colour multiplies it.
    pub(crate) base: Option<(Index<json::Texture>, bool)>,
    /// The normal texture.
    pub(crate) normal: Option<Index<json::Texture>>,
    /// The metallic-roughness texture and whether it carries the roughness and the metalness; a
    /// channel it lacks keeps the plain value.
    pub(crate) metallic_roughness: Option<(Index<json::Texture>, bool, bool)>,
}

/// Returns the name of the material of `slot`: the file's, or `material_<position>`, or `default`.
pub(crate) fn display_name(material: &Material, slot: Slot) -> String {
    if !material.name.is_empty() {
        return material.name.clone();
    }
    match slot {
        Slot::Library(position) => format!("material_{position}"),
        Slot::Default => "default".to_owned(),
    }
}

/// The alpha mode of `material`, its alpha factor and its alpha cutoff.
fn alpha_of(material: &Material) -> (AlphaMode, f32, Option<f32>) {
    let opacity = material.opacity.clamp(0.0, 1.0);
    let decal = is_one_of(&material.shader, &DECAL_SHADERS);
    let glass = is_one_of(&material.shader, &GLASS_SHADERS);
    if material.alpha_test > 0.0 {
        (
            AlphaMode::Mask,
            1.0,
            Some(material.alpha_test.clamp(0.0, 1.0)),
        )
    } else if glass {
        (AlphaMode::Blend, opacity.min(GLASS_ALPHA), None)
    } else if decal || opacity < 1.0 {
        (AlphaMode::Blend, opacity, None)
    } else {
        (AlphaMode::Opaque, 1.0, None)
    }
}

/// Returns whether `material` is drawn with transparency, so that the alpha channel of its
/// diffuse texture counts.
pub(crate) fn uses_alpha(material: &Material) -> bool {
    alpha_of(material).0 != AlphaMode::Opaque
}

/// Returns the glTF material that draws `material` with `textures` over its plain colours.
///
/// The diffuse colour multiplies a base colour texture that does not hold the colours already (one
/// made with a specular map does). A normal texture needs no factor. The factors of the
/// metallic-roughness texture are 1 for the channels it carries; for the others the plain value
/// stays. A metal whose base colour image lacks the colours, because it has no specular map, the
/// map failed to load or it has no diffuse map, keeps the plain metal of the colours mode: its
/// specular colour is the base colour factor, which a diffuse texture multiplies, and its
/// metalness is 1.
pub(crate) fn build_material(
    material: &Material,
    fallback_name: &str,
    textures: &MaterialTextures,
) -> json::Material {
    let (carries_roughness, carries_metalness) = textures
        .metallic_roughness
        .map_or((false, false), |(_, roughness, metallic)| {
            (roughness, metallic)
        });
    let tinted = textures.base.is_some_and(|(_, tinted)| tinted);
    let shade = shading(material);
    let plain_metal = shade.metal && !tinted;
    let colour = if tinted { [1.0; 3] } else { shade.colour };
    let (mode, alpha, cutoff) = alpha_of(material);
    let decal = is_one_of(&material.shader, &DECAL_SHADERS);
    let info = |index: Index<json::Texture>| json::texture::Info {
        index,
        tex_coord: 0,
        extensions: None,
        extras: json::extras::Void::default(),
    };
    json::Material {
        name: Some(if material.name.is_empty() {
            fallback_name.to_owned()
        } else {
            material.name.clone()
        }),
        alpha_mode: Valid(mode),
        alpha_cutoff: cutoff.map(AlphaCutoff),
        double_sided: decal || material.flags.two_sided(),
        pbr_metallic_roughness: PbrMetallicRoughness {
            base_color_factor: PbrBaseColorFactor([colour[0], colour[1], colour[2], alpha]),
            base_color_texture: textures.base.map(|(index, _)| info(index)),
            metallic_factor: StrengthFactor(if carries_metalness || plain_metal {
                1.0
            } else {
                0.0
            }),
            roughness_factor: StrengthFactor(if carries_roughness {
                1.0
            } else {
                shade.roughness
            }),
            metallic_roughness_texture: textures
                .metallic_roughness
                .map(|(index, _, _)| info(index)),
            ..PbrMetallicRoughness::default()
        },
        normal_texture: textures.normal.map(|index| json::material::NormalTexture {
            index,
            scale: 1.0,
            tex_coord: 0,
            extensions: None,
            extras: json::extras::Void::default(),
        }),
        emissive_factor: EmissiveFactor(emissive(material)),
        ..json::Material::default()
    }
}

fn is_one_of(shader: &str, names: &[&str]) -> bool {
    names.iter().any(|name| shader.eq_ignore_ascii_case(name))
}

/// Returns the base colour factor the GLB writer gives `material` without a specular map.
///
/// It is [`Shading::colour`](crate::Shading::colour): the specular colour of a metal, else the diffuse colour, clamped to
/// 0 to 1, for a layered material the blend of its layers; a diffuse texture is multiplied by it.
#[must_use]
pub fn base_colour_factor(material: &Material) -> [f32; 3] {
    shading(material).colour
}

/// The emissive colour times the glow, scaled into 0 to 1 by its brightest channel.
fn emissive(material: &Material) -> [f32; 3] {
    let glow = material.glow.max(0.0);
    let scaled = material.emissive.map(|channel| channel.max(0.0) * glow);
    let brightest = scaled.into_iter().fold(1.0_f32, f32::max);
    scaled.map(|channel| channel / brightest)
}
