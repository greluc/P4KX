// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::mtl_texture::{DIFFUSE_SLOT, texture_suffix};
use crate::{CompanionResolver, Material, MaterialLibrary};

/// The least value of every channel of a white diffuse colour.
const WHITE_FROM: f32 = 0.99;

impl Material {
    /// Returns whether the material is a parallax relief decal: a `MeshDecal` with
    /// `PARALLAX_OCCLUSION_MAPPING` in its generation mask and a displacement map (`_displ`),
    /// whose diffuse colour is white, which no tint mode colours (`ETintModeType` 0 or missing)
    /// and whose diffuse map holds no colour of its own.
    ///
    /// The diffuse map of such a decal is the white mask of the groove the displacement map cuts,
    /// so drawn as colour it paints contour lines the game shows as recesses. A decal with a
    /// colour of its own, a tint mode or a coloured diffuse map (buttons, stitching) keeps its
    /// colour. A diffuse map that was not checked, see [`Self::diffuse_map_is_grey`], counts as
    /// grey.
    #[must_use]
    pub fn is_relief_decal(&self) -> bool {
        self.is_relief_decal_candidate() && self.diffuse_map_is_grey != Some(false)
    }

    pub(crate) fn is_relief_decal_candidate(&self) -> bool {
        self.shader.eq_ignore_ascii_case("MeshDecal")
            && self
                .string_gen_mask
                .split('%')
                .any(|flag| flag == "PARALLAX_OCCLUSION_MAPPING")
            && self.diffuse.iter().all(|channel| *channel >= WHITE_FROM)
            && self
                .public_param("ETintModeType")
                .is_none_or(|mode| mode[0] == 0.0)
            && self.textures.iter().any(|texture| {
                let suffix = texture_suffix(&texture.file);
                suffix.eq_ignore_ascii_case("displ") || suffix.eq_ignore_ascii_case("disp")
            })
    }

    fn diffuse_file(&self) -> Option<&str> {
        self.textures
            .iter()
            .find(|texture| texture.map.eq_ignore_ascii_case(DIFFUSE_SLOT))
            .map(|texture| texture.file.as_str())
    }
}

/// Asks `companions` whether the diffuse map of each relief-decal candidate of `library` is grey.
pub(crate) fn check_diffuse_maps(
    library: &mut MaterialLibrary,
    companions: &mut dyn CompanionResolver,
) {
    for material in &mut library.materials {
        if material.is_relief_decal_candidate()
            && let Some(file) = material.diffuse_file()
        {
            material.diffuse_map_is_grey = companions.texture_is_grey(file);
        }
    }
}
