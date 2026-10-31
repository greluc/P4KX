// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The one rule for which materials read the vertex colours of their mesh.

use crate::Material;
use crate::mtl_roles::TextureRole;

const PER_VERTEX_TINT: &str = "PER_VERTEX_TINT";

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
        if self.is_glass() {
            return self
                .string_gen_mask
                .split('%')
                .any(|flag| flag == PER_VERTEX_TINT);
        }
        self.is_surface_shader() && self.blends_layers_by_vertex_colours()
    }

    /// Returns whether at least one layer is blended over a base without a blend map: the layers
    /// after the first, plus the first too when the material has a diffuse map of its own.
    fn blends_layers_by_vertex_colours(&self) -> bool {
        let has_role =
            |wanted: &[TextureRole]| self.file_roles().any(|role| wanted.contains(&role));
        let blend_map = has_role(&[TextureRole::Blend, TextureRole::Mask]);
        let base = usize::from(has_role(&[TextureRole::Diffuse]));
        !blend_map && self.layers.len() + base >= 2
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
        assert!(!reads("GlassPBR", "%NORMAL_MAP", &[], 0)?);
        assert!(!reads("GlassPBR", "%NOT_PER_VERTEX_TINT_AT_ALL", &[], 0)?);
        Ok(())
    }

    #[test]
    fn every_surface_shader_reads_them_without_a_blend_map() -> Result {
        for shader in [
            "HardSurface",
            "Illum",
            "LayerBlend",
            "LayerBlend_V2",
            "layer",
            "Simple",
        ] {
            assert!(reads(shader, "", &[], 2)?, "{shader}");
            assert!(
                !reads(shader, "", &[("TexSlot8", "a/b_blend.tif")], 2)?,
                "{shader} with a blend map"
            );
            assert!(
                !reads(shader, "", &[("TexSlot9", "a/b_mask.tif")], 2)?,
                "{shader} with a mask map"
            );
        }
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
    fn a_placeholder_is_no_diffuse_map_and_no_blend_map() -> Result {
        assert!(!reads(
            "HardSurface",
            "",
            &[("TexSlot1", "$RenderToTexture")],
            1
        )?);
        assert!(!reads(
            "HardSurface",
            "",
            &[("TexSlot1", "  $TintPaletteDecal")],
            1
        )?);
        assert!(reads("HardSurface", "", &[("TexSlot8", "$b_blend")], 2)?);
        assert!(!reads(
            "HardSurface",
            "",
            &[("TexSlot8", " a/b_blend.tif")],
            2
        )?);
        assert!(reads(
            "HardSurface",
            "",
            &[("TexSlot1", " a/b_diff.tif ")],
            1
        )?);
        Ok(())
    }

    #[test]
    fn other_shaders_never_read_them() -> Result {
        assert!(!reads("MeshDecal", "%PER_VERTEX_TINT", &[], 3)?);
        assert!(!reads("Hologram", "", &[], 3)?);
        assert!(!reads("Glass", "%PER_VERTEX_TINT", &[], 0)?);
        Ok(())
    }
}
