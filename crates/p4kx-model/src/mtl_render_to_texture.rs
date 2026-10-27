// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::Material;

const DIFFUSE_SLOT: &str = "TexSlot1";
const RENDER_TO_TEXTURE: &str = "$RenderToTexture";

impl Material {
    /// Returns whether the material is a render-to-texture decal: a `MeshDecal` whose diffuse map
    /// (`TexSlot1`) is the engine placeholder `$RenderToTexture`.
    ///
    /// The game fills that map at runtime (registration marks, name plates, serial numbers), so
    /// the Archive holds nothing to draw and the decal would show as a white strip. Screens and
    /// HUD planes name the placeholder in another slot of another shader and are not affected.
    #[must_use]
    pub fn is_render_to_texture_decal(&self) -> bool {
        self.shader.eq_ignore_ascii_case("MeshDecal")
            && self.textures.iter().any(|texture| {
                texture.map.eq_ignore_ascii_case(DIFFUSE_SLOT)
                    && texture.file.eq_ignore_ascii_case(RENDER_TO_TEXTURE)
            })
    }
}
