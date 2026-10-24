// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};

use p4kx_extraction::model::Material;
use p4kx_extraction::{base_colour_factor, diffuse_texture_path};
use p4kx_settings::ModelMaterials;

use super::build::SharedMesh;
use super::pieces::Piece;

const GLASS_ALPHA: f32 = 0.3;
const DECAL_SHADERS: [&str; 2] = ["MeshDecal", "Decal"];
const GLASS_SHADERS: [&str; 1] = ["GlassPBR"];

/// The material library, palette and material ID that decide how a range is drawn.
pub(super) type SurfaceKey = (usize, usize, u16);

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

fn address<T>(value: &T) -> usize {
    std::ptr::from_ref(value) as usize
}

pub(super) fn surface_key(part: &Piece<'_>, id: u16) -> SurfaceKey {
    (address(part.materials), part.palette.map_or(0, address), id)
}

/// Resolves the material `id` of `part` the way the GLB export picks it: the sub-material of
/// its library, else the engine's default, with the part's tint palette applied. Returns `None`
/// when nothing is drawn with it: a hidden material or a decal, whose shape only a texture gives.
pub(super) fn resolve(part: &Piece<'_>, id: u16, mode: ModelMaterials) -> Option<Surface> {
    let fallback = Material::engine_default();
    let library = part.materials.library();
    let found = library
        .and_then(|library| {
            let position = if library.single { 0 } else { usize::from(id) };
            library.materials.get(position)
        })
        .unwrap_or(&fallback);
    if found.hidden() || is_one_of(&found.shader, &DECAL_SHADERS) {
        return None;
    }
    let tinted = part.palette.and_then(|palette| palette.apply(found));
    surface_of(tinted.as_ref().unwrap_or(found), mode)
}

/// Returns how `material` is drawn: the base colour the GLB writer uses (the specular colour of a
/// metal, else the diffuse colour), its opacity, and in the textures mode its diffuse texture.
pub(super) fn surface_of(material: &Material, mode: ModelMaterials) -> Option<Surface> {
    if is_one_of(&material.shader, &DECAL_SHADERS) {
        return None;
    }
    let opacity = material.opacity.clamp(0.0, 1.0);
    let glass = is_one_of(&material.shader, &GLASS_SHADERS);
    let (alpha, translucent) = if glass {
        (opacity.min(GLASS_ALPHA), true)
    } else if opacity < 1.0 && material.alpha_test <= 0.0 {
        (opacity, true)
    } else {
        (1.0, false)
    };
    let [red, green, blue] = base_colour_factor(material);
    Some(Surface {
        colour: [red, green, blue, alpha],
        translucent,
        texture: (mode == ModelMaterials::Textures)
            .then(|| diffuse_texture_path(material))
            .flatten(),
    })
}

/// Resolves the surface of every material range the parts use, once per library, palette and
/// material ID, and lists the distinct texture paths they want.
pub(super) fn surfaces_of(
    pieces: &[Piece<'_>],
    model_of: &[Option<usize>],
    meshes: &[Option<SharedMesh>],
    mode: ModelMaterials,
) -> (HashMap<SurfaceKey, Option<Surface>>, Vec<String>) {
    let mut surfaces = HashMap::new();
    let mut wanted: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (at, part) in pieces.iter().enumerate() {
        let mesh = model_of
            .get(at)
            .copied()
            .flatten()
            .and_then(|id| meshes.get(id))
            .and_then(Option::as_ref);
        let Some(mesh) = mesh else {
            continue;
        };
        for range in &mesh.ranges {
            surfaces
                .entry(surface_key(part, range.material))
                .or_insert_with(|| {
                    let surface = resolve(part, range.material, mode);
                    if let Some(path) = surface.as_ref().and_then(|s| s.texture.as_ref())
                        && seen.insert(path.clone())
                    {
                        wanted.push(path.clone());
                    }
                    surface
                });
        }
    }
    (surfaces, wanted)
}

#[cfg(test)]
mod tests {
    use p4kx_extraction::model::Material;
    use p4kx_settings::ModelMaterials;

    use super::surface_of;

    fn plain(diffuse: [f32; 3], specular: [f32; 3], surface: &str) -> Material {
        let mut material = Material::engine_default();
        material.diffuse = diffuse;
        material.specular = specular;
        material.surface_type = surface.to_owned();
        material
    }

    #[test]
    fn a_metal_with_a_dark_diffuse_and_a_bright_specular_is_drawn_in_its_specular_colour() {
        let metal = plain([0.02; 3], [0.8, 0.7, 0.6], "metal_dense");

        let surface = surface_of(&metal, ModelMaterials::Colours);

        assert_eq!(
            surface.map(|surface| surface.colour),
            Some([0.8, 0.7, 0.6, 1.0]),
            "the colour the GLB writer takes"
        );
    }

    #[test]
    fn a_painted_surface_keeps_its_diffuse_colour() {
        let paint = plain([0.3, 0.4, 0.5], [0.04; 3], "");

        let surface = surface_of(&paint, ModelMaterials::Colours);

        assert_eq!(
            surface.map(|surface| surface.colour),
            Some([0.3, 0.4, 0.5, 1.0]),
            "diffuse"
        );
    }

    #[test]
    fn glass_is_translucent_and_a_decal_is_not_drawn() {
        let mut glass = plain([0.1; 3], [0.04; 3], "");
        glass.shader = "GlassPBR".to_owned();
        let mut decal = plain([0.1; 3], [0.04; 3], "");
        decal.shader = "MeshDecal".to_owned();

        assert!(
            surface_of(&glass, ModelMaterials::Colours).is_some_and(|s| s.translucent),
            "glass blends"
        );
        assert!(
            surface_of(&decal, ModelMaterials::Colours).is_none(),
            "decals are left out"
        );
    }
}
