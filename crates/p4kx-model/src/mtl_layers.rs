// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};

use crate::mtl_load::request_path;
use crate::mtl_parse::{scalar, vector};
use crate::{
    CompanionPath, CompanionResolver, Material, MaterialLayer, MaterialLibrary, TextureSlot,
};

/// The most distinct layer files one material library may make the reader fetch; layers beyond
/// are reported as unresolved.
pub const MAX_LAYER_FILES: usize = 256;

const WHITE: [f32; 3] = [1.0; 3];

/// The surface values of the `.mtl` file a [`MaterialLayer`] names; colours are linear.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LayerSurface {
    /// The layer's diffuse colour, which its tint multiplies.
    pub diffuse: [f32; 3],
    /// The layer's specular colour.
    pub specular: [f32; 3],
    /// The texture slots of the layer's own file, in file order: its diffuse map in `TexSlot1`
    /// and its normal map in `TexSlot2`, with their `TexMod` tiling.
    pub textures: Vec<TextureSlot>,
}

impl LayerSurface {
    fn of(material: &Material) -> Self {
        Self {
            diffuse: material.diffuse,
            specular: material.specular,
            textures: material.textures.clone(),
        }
    }
}

impl MaterialLayer {
    /// Returns the layer's `TintColor`, linear; white when it is missing or malformed.
    #[must_use]
    pub fn tint(&self) -> [f32; 3] {
        self.attribute("TintColor")
            .and_then(vector)
            .unwrap_or(WHITE)
    }

    /// Returns the layer's `GlossMult`, 1 when it is missing or malformed.
    #[must_use]
    pub fn gloss_mult(&self) -> f32 {
        self.attribute("GlossMult").and_then(scalar).unwrap_or(1.0)
    }
}

impl Material {
    /// Returns the `WearBlendBase` of the material's `PublicParams`, the share of the wear layers
    /// in the blend of its layers; 0 when it is missing or malformed.
    #[must_use]
    pub fn wear_blend(&self) -> f32 {
        self.public_params
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("WearBlendBase"))
            .and_then(|(_, value)| scalar(value))
            .unwrap_or(0.0)
    }
}

/// Reads the `.mtl` file of every layer of `library` through `companions` and records its
/// surface values in the layer; the paths of the files that are missing or unreadable are listed
/// in [`MaterialLibrary::unresolved_layers`].
pub(crate) fn resolve(library: &mut MaterialLibrary, companions: &mut dyn CompanionResolver) {
    let mut files: HashMap<String, Option<MaterialLibrary>> = HashMap::new();
    let mut unresolved: Vec<String> = Vec::new();
    let mut listed: HashSet<String> = HashSet::new();
    for material in &mut library.materials {
        for layer in &mut material.layers {
            let Some(path) = layer
                .attribute("Path")
                .map(str::trim)
                .filter(|p| !p.is_empty())
            else {
                continue;
            };
            let path = path.to_owned();
            let submaterial = layer.attribute("Submtl").map(str::trim).unwrap_or_default();
            let surface = request_path(&path)
                .and_then(|request| {
                    let key = request.to_ascii_lowercase();
                    if !files.contains_key(&key) {
                        let file = if files.len() < MAX_LAYER_FILES {
                            fetch(&request, companions)
                        } else {
                            None
                        };
                        files.insert(key.clone(), file);
                    }
                    files
                        .get(&key)?
                        .as_ref()
                        .and_then(|file| pick(file, submaterial))
                })
                .map(LayerSurface::of);
            if surface.is_none() && unresolved.len() < MAX_LAYER_FILES {
                let normal = format!(
                    "{}|{}",
                    path.replace('\\', "/").to_ascii_lowercase(),
                    submaterial.to_ascii_lowercase()
                );
                if listed.insert(normal) {
                    unresolved.push(path);
                }
            }
            layer.surface = surface;
        }
    }
    library.unresolved_layers = unresolved;
}

fn fetch(request: &str, companions: &mut dyn CompanionResolver) -> Option<MaterialLibrary> {
    let bytes = companions
        .resolve(CompanionPath::Material(request))
        .ok()
        .flatten()?;
    MaterialLibrary::parse(request, &bytes).ok()
}

fn pick<'f>(file: &'f MaterialLibrary, submaterial: &str) -> Option<&'f Material> {
    if file.single || submaterial.is_empty() {
        file.materials.first()
    } else {
        file.materials
            .iter()
            .find(|material| material.name.eq_ignore_ascii_case(submaterial))
    }
}
