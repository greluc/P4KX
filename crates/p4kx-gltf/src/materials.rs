use std::collections::{BTreeMap, BTreeSet};

use gltf_json as json;
use json::material::{
    AlphaCutoff, AlphaMode, EmissiveFactor, PbrBaseColorFactor, PbrMetallicRoughness,
    StrengthFactor,
};
use json::validation::Checked::Valid;
use p4kx_model::{Lod, Material, Model};

const GLASS_ALPHA: f32 = 0.3;
const METAL_SPECULAR: f32 = 0.5;
const DARK_DIFFUSE: f32 = 0.2;
const TEXTURE_GLOSS_ROUGHNESS: f32 = 0.5;
const MAX_SHININESS: f32 = 255.0;

const DECAL_SHADERS: [&str; 2] = ["MeshDecal", "Decal"];
const GLASS_SHADERS: [&str; 1] = ["GlassPBR"];
const TEXTURE_GLOSS_SHADERS: [&str; 3] = ["HardSurface", "LayerBlend_V2", "Layer"];

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
    model: &'m Model,
    slots: BTreeMap<u16, Option<Slot>>,
    default: Material,
}

impl<'m> MaterialPlan<'m> {
    /// Resolves every material ID `lod` uses.
    pub(crate) fn new(model: &'m Model, lod: &Lod) -> Self {
        let ids: BTreeSet<u16> = lod
            .meshes
            .iter()
            .flat_map(|mesh| &mesh.submeshes)
            .map(|submesh| submesh.material_id)
            .collect();
        let library = model.materials.library();
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
            model,
            slots,
            default: Material::engine_default(),
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
                .model
                .materials
                .library()
                .and_then(|library| library.materials.get(position))
                .unwrap_or(&self.default),
            Slot::Default => &self.default,
        }
    }
}

fn resolve(library: &p4kx_model::MaterialLibrary, id: u16) -> Option<Slot> {
    let position = if library.single { 0 } else { usize::from(id) };
    match library.materials.get(position) {
        Some(material) if material.hidden() => None,
        Some(_) => Some(Slot::Library(position)),
        None => Some(Slot::Default),
    }
}

/// Returns the glTF material that draws `material` as plain colours.
pub(crate) fn colour_material(material: &Material, fallback_name: &str) -> json::Material {
    let metallic = is_metal(material);
    let colour = if metallic {
        material.specular
    } else {
        material.diffuse
    };
    let colour = colour.map(|channel| channel.clamp(0.0, 1.0));
    let opacity = material.opacity.clamp(0.0, 1.0);
    let decal = is_one_of(&material.shader, &DECAL_SHADERS);
    let glass = is_one_of(&material.shader, &GLASS_SHADERS);
    let (mode, alpha, cutoff) = if material.alpha_test > 0.0 {
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
    };
    let roughness = if is_one_of(&material.shader, &TEXTURE_GLOSS_SHADERS) {
        TEXTURE_GLOSS_ROUGHNESS
    } else {
        1.0 - (material.shininess / MAX_SHININESS).clamp(0.0, 1.0)
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
            metallic_factor: StrengthFactor(if metallic { 1.0 } else { 0.0 }),
            roughness_factor: StrengthFactor(roughness),
            ..PbrMetallicRoughness::default()
        },
        emissive_factor: EmissiveFactor(emissive(material)),
        ..json::Material::default()
    }
}

fn is_one_of(shader: &str, names: &[&str]) -> bool {
    names.iter().any(|name| shader.eq_ignore_ascii_case(name))
}

fn luminance([red, green, blue]: [f32; 3]) -> f32 {
    0.0722_f32.mul_add(blue, 0.2126_f32.mul_add(red, 0.7152 * green))
}

/// Metals have a bright specular colour, and either a metal surface type or a dark diffuse colour.
fn is_metal(material: &Material) -> bool {
    let metal_surface = material
        .surface_type
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("metal"));
    luminance(material.specular) >= METAL_SPECULAR
        && (metal_surface || luminance(material.diffuse) < DARK_DIFFUSE)
}

/// The emissive colour times the glow, scaled into 0 to 1 by its brightest channel.
fn emissive(material: &Material) -> [f32; 3] {
    let glow = material.glow.max(0.0);
    let scaled = material.emissive.map(|channel| channel.max(0.0) * glow);
    let brightest = scaled.into_iter().fold(1.0_f32, f32::max);
    scaled.map(|channel| channel / brightest)
}
