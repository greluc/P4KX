// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::Material;

/// The glass shader; no other shader of the Archive draws glass.
pub(crate) const GLASS_SHADER: &str = "GlassPBR";

/// The shaders that draw a surface and take layers from `MatLayers`.
const SURFACE_SHADERS: [&str; 6] = [
    "HardSurface",
    "Illum",
    "LayerBlend",
    "LayerBlend_V2",
    "Layer",
    "Simple",
];

impl Material {
    /// Returns whether the material is drawn with the glass shader.
    #[must_use]
    pub fn is_glass(&self) -> bool {
        self.shader.eq_ignore_ascii_case(GLASS_SHADER)
    }

    /// Returns whether the material is drawn with a surface shader, the shaders whose layers are
    /// blended into one surface.
    #[must_use]
    pub fn is_surface_shader(&self) -> bool {
        SURFACE_SHADERS
            .iter()
            .any(|name| self.shader.eq_ignore_ascii_case(name))
    }
}
