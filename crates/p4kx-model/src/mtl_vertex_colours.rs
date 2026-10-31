// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The one rule for which materials read the vertex colours of their mesh.

use crate::Material;
use crate::mtl_texture::texture_suffix_lowercase;

const PER_VERTEX_TINT: &str = "PER_VERTEX_TINT";
const GLASS_SHADERS: [&str; 2] = ["GlassPBR", "Glass"];
const SURFACE_SHADERS: [&str; 6] = [
    "HardSurface",
    "Illum",
    "LayerBlend",
    "LayerBlend_V2",
    "Layer",
    "Simple",
];

/// Texture suffixes of the maps that blend layers: a layered material with one takes its layer
/// shares from it, not from the vertex colours.
const BLEND_SUFFIXES: [&str; 3] = ["blend", "mask", "opac"];

/// Texture suffixes of the maps in `TexSlot1` that are no diffuse map.
const OTHER_MAP_SUFFIXES: [&str; 17] = [
    "ddna", "ddn", "spec", "mask", "opac", "blend", "detail", "wear", "ao", "displ", "disp",
    "gloss", "id", "scalp", "dir", "dirm", "hal",
];

impl Material {
    /// Returns whether the material reads the vertex colours of its mesh: glass whose generation
    /// mask names `PER_VERTEX_TINT`, and a layered surface that has no blend map (a `_blend`,
    /// `_mask` or `_opac` texture), whose layers then take their share from them.
    ///
    /// The GLB writer writes `COLOR_0` for the primitives of such materials only, and the package
    /// states the result as `reads_vertex_colours` for the Blender add-on, which builds its
    /// vertex-colour nodes from it; the colours of any other material are mask data.
    #[must_use]
    pub fn reads_vertex_colours(&self) -> bool {
        let named = |names: &[&str]| {
            names
                .iter()
                .any(|name| self.shader.eq_ignore_ascii_case(name))
        };
        if named(&GLASS_SHADERS) {
            return self
                .string_gen_mask
                .split('%')
                .any(|flag| flag == PER_VERTEX_TINT);
        }
        named(&SURFACE_SHADERS) && self.blends_layers_by_vertex_colours()
    }

    /// Returns whether at least one layer is blended over a base without a blend map: the layers
    /// after the first, plus the first too when the material has a diffuse map of its own.
    fn blends_layers_by_vertex_colours(&self) -> bool {
        let blend_map = self.textures.iter().any(|texture| {
            let file = texture.file.trim_start();
            !file.starts_with('$')
                && BLEND_SUFFIXES.contains(&texture_suffix_lowercase(file).as_str())
        });
        let base = usize::from(self.has_diffuse_map());
        !blend_map && self.layers.len() + base >= 2
    }

    fn has_diffuse_map(&self) -> bool {
        self.diffuse_file().is_some_and(|file| {
            !file.is_empty()
                && !OTHER_MAP_SUFFIXES.contains(&texture_suffix_lowercase(file).as_str())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use crate::MaterialLibrary;

    type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn reads(shader: &str, mask: &str, textures: &[(&str, &str)], layers: usize) -> Result<bool> {
        let mut slots = String::new();
        for (slot, file) in textures {
            write!(slots, r#"<Texture Map="{slot}" File="{file}"/>"#)?;
        }
        let mut layer_list = String::new();
        for at in 0..layers {
            write!(layer_list, r#"<Layer Name="l{at}" Path="a/l{at}.mtl"/>"#)?;
        }
        let xml = format!(
            r#"<Material><SubMaterials><Material Name="m" Shader="{shader}" StringGenMask="{mask}">
            <Textures>{slots}</Textures><MatLayers>{layer_list}</MatLayers></Material></SubMaterials></Material>"#
        );
        let library = MaterialLibrary::parse("a", xml.as_bytes())?;
        let material = library.materials.first().ok_or("a material")?;
        Ok(material.reads_vertex_colours())
    }

    #[test]
    fn glass_reads_them_with_per_vertex_tint_only() -> Result {
        assert!(reads("GlassPBR", "%NORMAL_MAP%PER_VERTEX_TINT", &[], 0)?);
        assert!(reads("Glass", "%PER_VERTEX_TINT", &[], 0)?);
        assert!(!reads("GlassPBR", "%NORMAL_MAP", &[], 0)?);
        Ok(())
    }

    #[test]
    fn a_layered_surface_reads_them_without_a_blend_map() -> Result {
        assert!(reads("HardSurface", "", &[], 2)?);
        assert!(!reads(
            "HardSurface",
            "",
            &[("TexSlot8", "a/b_blend.tif")],
            2
        )?);
        assert!(!reads(
            "HardSurface",
            "",
            &[("TexSlot9", "a/b_mask.tif")],
            2
        )?);
        Ok(())
    }

    #[test]
    fn a_surface_needs_a_layer_to_blend() -> Result {
        assert!(!reads("HardSurface", "", &[], 1)?);
        assert!(!reads("HardSurface", "", &[], 0)?);
        assert!(reads(
            "HardSurface",
            "",
            &[("TexSlot1", "a/b_diff.tif")],
            1
        )?);
        assert!(!reads(
            "HardSurface",
            "",
            &[("TexSlot2", "a/b_ddna.tif")],
            1
        )?);
        Ok(())
    }

    #[test]
    fn other_shaders_never_read_them() -> Result {
        assert!(!reads("MeshDecal", "%PER_VERTEX_TINT", &[], 3)?);
        assert!(!reads("Hologram", "", &[], 3)?);
        Ok(())
    }
}
