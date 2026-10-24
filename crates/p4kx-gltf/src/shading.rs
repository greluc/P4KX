// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{LayerSurface, Material, MaterialLayer};

use crate::texture_pixels::metal_share;

const METAL_SPECULAR: f32 = 0.5;
const DARK_DIFFUSE: f32 = 0.2;
const TEXTURE_GLOSS_ROUGHNESS: f32 = 0.5;
const MAX_SHININESS: f32 = 255.0;

const TEXTURE_GLOSS_SHADERS: [&str; 3] = ["HardSurface", "LayerBlend_V2", "Layer"];

/// How a material is shaded without textures: the colours, whether it is a metal and how rough it
/// is.
///
/// The GLB writer and the viewers take their plain colours from here, so that what is seen is
/// what is exported.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Shading {
    /// The base colour, linear, clamped to 0 to 1: the specular colour of a metal, else the
    /// diffuse colour; for a layered material the weighted blend of the layers' colours, each
    /// moving from its tinted diffuse towards its specular colour by its metal share.
    pub colour: [f32; 3],
    /// Whether the surface is a metal.
    pub metal: bool,
    /// The roughness from 0 to 1.
    pub roughness: f32,
    /// The diffuse colour, linear, as the material gives it or its layers blend it.
    pub diffuse: [f32; 3],
    /// The specular colour, linear, as the material gives it or its layers blend it.
    pub specular: [f32; 3],
}

/// Returns how `material` is shaded.
///
/// A material with `MatLayers` whose layer files were read (see
/// [`MaterialStatus::load`](p4kx_model::MaterialStatus::load)) is shaded by its layers: each
/// layer has its own file's diffuse colour times the layer's tint, or the palette entry of its
/// `PaletteTint` with that entry's specular colour and glossiness, all clamped to 0 to 1 first.
/// The layers are weighted by position: the first with `1 - WearBlendBase`, the others with
/// `WearBlendBase` between them. A layer whose file is missing has no weight; when nothing
/// resolved has weight, the material is shaded by its own colours, as is any material without
/// layers.
#[must_use]
pub fn shading(material: &Material) -> Shading {
    layered(material).unwrap_or_else(|| plain(material))
}

fn plain(material: &Material) -> Shading {
    let metal = is_metal(material);
    let colour = if metal {
        material.specular
    } else {
        material.diffuse
    };
    Shading {
        colour: colour.map(|channel| channel.clamp(0.0, 1.0)),
        metal,
        roughness: plain_roughness(material),
        diffuse: material.diffuse,
        specular: material.specular,
    }
}

/// Returns the roughness of `material` from its shininess, or half where gloss comes from
/// textures and layers; a palette gives the glossiness of its paint.
fn plain_roughness(material: &Material) -> f32 {
    let texture_gloss = TEXTURE_GLOSS_SHADERS
        .iter()
        .any(|name| material.shader.eq_ignore_ascii_case(name));
    if texture_gloss && !material.tinted {
        TEXTURE_GLOSS_ROUGHNESS
    } else {
        1.0 - (material.shininess / MAX_SHININESS).clamp(0.0, 1.0)
    }
}

fn luminance([red, green, blue]: [f32; 3]) -> f32 {
    0.0722_f32.mul_add(blue, 0.2126_f32.mul_add(red, 0.7152 * green))
}

/// Metals have a bright specular colour, and either a metal surface type or a dark diffuse colour.
pub(crate) fn is_metal(material: &Material) -> bool {
    let named = material
        .surface_type
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("metal"));
    luminance(material.specular) >= METAL_SPECULAR
        && (named || luminance(material.diffuse) < DARK_DIFFUSE)
}

struct Layer {
    diffuse: [f32; 3],
    specular: [f32; 3],
    colour: [f32; 3],
    metal: f32,
    roughness: f32,
}

fn unit(colour: [f32; 3]) -> [f32; 3] {
    colour.map(|channel| channel.clamp(0.0, 1.0))
}

fn shade(layer: &MaterialLayer, surface: &LayerSurface) -> Layer {
    let (tint, specular, smoothness) = layer.palette_entry.map_or_else(
        || {
            (
                layer.tint(),
                surface.specular,
                1.0 - TEXTURE_GLOSS_ROUGHNESS,
            )
        },
        |entry| (entry.tint, entry.specular, entry.glossiness / MAX_SHININESS),
    );
    let [red, green, blue] = unit(surface.diffuse);
    let [tint_red, tint_green, tint_blue] = unit(tint);
    let diffuse = [red * tint_red, green * tint_green, blue * tint_blue];
    let specular = unit(specular);
    let metal = metal_share(specular);
    let [diffuse_red, diffuse_green, diffuse_blue] = diffuse;
    let [specular_red, specular_green, specular_blue] = specular;
    let towards = |from: f32, to: f32| (to - from).mul_add(metal, from);
    Layer {
        diffuse,
        specular,
        colour: [
            towards(diffuse_red, specular_red),
            towards(diffuse_green, specular_green),
            towards(diffuse_blue, specular_blue),
        ],
        metal,
        roughness: smoothness
            .clamp(0.0, 1.0)
            .mul_add(-layer.gloss_mult().clamp(0.0, 1.0), 1.0),
    }
}
fn mix(into: &mut [f32; 3], from: [f32; 3], share: f32) {
    for (target, value) in into.iter_mut().zip(from) {
        *target = value.mul_add(share, *target);
    }
}

fn layered(material: &Material) -> Option<Shading> {
    let layers = &material.layers;
    let wear = material.wear_blend().clamp(0.0, 1.0);
    let others = layers.len().saturating_sub(1).max(1);
    #[expect(clippy::cast_precision_loss, reason = "at most 64 layers")]
    let others = others as f32;
    let weight = |at: usize| if at == 0 { 1.0 - wear } else { wear / others };
    let resolved = || {
        layers
            .iter()
            .enumerate()
            .filter_map(|(at, layer)| Some((at, layer, layer.surface.as_ref()?)))
    };
    let (first, _, _) = resolved().next()?;
    let total: f32 = resolved().map(|(at, _, _)| weight(at)).sum();
    let only = total <= f32::EPSILON;
    if only && first != 0 {
        return None;
    }
    let mut mixed = Shading {
        colour: [0.0; 3],
        metal: false,
        roughness: 0.0,
        diffuse: [0.0; 3],
        specular: [0.0; 3],
    };
    let mut metal_total = 0.0_f32;
    for (at, layer, surface) in resolved() {
        let share = if only {
            f32::from(u8::from(at == first))
        } else {
            weight(at) / total
        };
        if share <= 0.0 {
            continue;
        }
        let shaded = shade(layer, surface);
        mix(&mut mixed.colour, shaded.colour, share);
        mix(&mut mixed.diffuse, shaded.diffuse, share);
        mix(&mut mixed.specular, shaded.specular, share);
        mixed.roughness = shaded.roughness.mul_add(share, mixed.roughness);
        metal_total = shaded.metal.mul_add(share, metal_total);
    }
    mixed.colour = mixed.colour.map(|channel| channel.clamp(0.0, 1.0));
    mixed.metal = metal_total >= 0.5;
    Some(mixed)
}
