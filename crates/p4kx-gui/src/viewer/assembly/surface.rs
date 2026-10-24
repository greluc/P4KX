// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::Part;
use p4kx_extraction::diffuse_texture_path;
use p4kx_extraction::model::Material;
use p4kx_settings::ModelMaterials;

const GLASS_ALPHA: f32 = 0.3;
const DECAL_SHADERS: [&str; 2] = ["MeshDecal", "Decal"];
const GLASS_SHADERS: [&str; 1] = ["GlassPBR"];

/// How one material ID of a part is drawn.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Surface {
    pub(super) colour: [f32; 4],
    pub(super) translucent: bool,
    pub(super) texture: Option<String>,
}

fn is_one_of(shader: &str, names: &[&str]) -> bool {
    names.iter().any(|name| shader.eq_ignore_ascii_case(name))
}

/// Resolves the material `id` of `part` the way the GLB export picks it: the sub-material of
/// its library, else the engine's default, with the part's tint palette applied. Returns `None`
/// when nothing is drawn with it: a hidden material or a decal, whose shape only a texture gives.
pub(super) fn resolve(part: &Part, id: u16, mode: ModelMaterials) -> Option<Surface> {
    let fallback = Material::engine_default();
    let library = part.materials().library();
    let found = library
        .and_then(|library| {
            let position = if library.single { 0 } else { usize::from(id) };
            library.materials.get(position)
        })
        .unwrap_or(&fallback);
    if found.hidden() || is_one_of(&found.shader, &DECAL_SHADERS) {
        return None;
    }
    let tinted = part.palette().and_then(|palette| palette.apply(found));
    let material = tinted.as_ref().unwrap_or(found);
    let opacity = material.opacity.clamp(0.0, 1.0);
    let glass = is_one_of(&material.shader, &GLASS_SHADERS);
    let (alpha, translucent) = if glass {
        (opacity.min(GLASS_ALPHA), true)
    } else if opacity < 1.0 && material.alpha_test <= 0.0 {
        (opacity, true)
    } else {
        (1.0, false)
    };
    let [red, green, blue] = material.diffuse.map(|channel| channel.clamp(0.0, 1.0));
    Some(Surface {
        colour: [red, green, blue, alpha],
        translucent,
        texture: (mode == ModelMaterials::Textures)
            .then(|| diffuse_texture_path(material))
            .flatten(),
    })
}
