// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::frame::linear;

const MAX_GLOW: f32 = 4.0;

/// How the instances of an Assembly are shaded.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Shading {
    /// Lit surfaces in the colour of their materials, as the viewer draws them.
    #[default]
    Solid,
    /// A transparent tinted shell that glows along its edges, drawn on the nearest surface only.
    Hologram(Hologram),
}

/// The look of the hologram shading pass.
///
/// The body of the shell is the tint at the `opacity`, a little brighter where a surface faces
/// the light; towards the silhouette and the creases the tint glows and turns opaque. The picture
/// holds premultiplied colours, which a read-back turns into straight alpha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hologram {
    /// The colour of the shell as sRGB bytes.
    pub tint: [u8; 3],
    /// The opacity of the body of the shell, 0 to 1.
    pub opacity: f32,
    /// The strength of the edge glow, 0 to 4.
    pub glow: f32,
}

impl Default for Hologram {
    fn default() -> Self {
        Self {
            tint: [0x4d, 0xc7, 0xff],
            opacity: 0.22,
            glow: 1.0,
        }
    }
}

impl Shading {
    /// Returns whether the shading leaves the picture with premultiplied colours.
    #[must_use]
    pub const fn is_premultiplied(&self) -> bool {
        matches!(self, Self::Hologram(_))
    }

    /// Returns the two rows after the camera in the uniform buffer: the tint in linear colour
    /// with the glow, and the opacity; zero for solid shading.
    pub(super) fn rows(&self) -> [[f32; 4]; 2] {
        match self {
            Self::Solid => [[0.0; 4]; 2],
            Self::Hologram(look) => {
                let [red, green, blue] = linear(look.tint);
                let finite = |value: f32, fallback: f32| {
                    if value.is_finite() { value } else { fallback }
                };
                let defaults = Hologram::default();
                [
                    [
                        red,
                        green,
                        blue,
                        finite(look.glow, defaults.glow).clamp(0.0, MAX_GLOW),
                    ],
                    [
                        finite(look.opacity, defaults.opacity).clamp(0.0, 1.0),
                        0.0,
                        0.0,
                        0.0,
                    ],
                ]
            }
        }
    }
}
