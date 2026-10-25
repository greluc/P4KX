// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::frame::linear;

const MAX_GLOW: f32 = 4.0;
const EDGE_WEIGHT: f32 = 0.2;

/// How the instances of an Assembly are shaded.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Shading {
    /// Lit surfaces in the colour of their materials, as the viewer draws them.
    #[default]
    Solid,
    /// A see-through tinted shell that glows along its edges, drawn from every layer of the model.
    Hologram(Hologram),
}

/// The look of the hologram shading pass.
///
/// Every surface of the model is drawn, the deeper ones weaker, so the inner structure and the far
/// side show through the near side. The `opacity` and the `glow` set how opaque the body and the
/// edges of the shell are, and together they cap the opacity the layers can add up to; towards the
/// silhouette the tint glows and turns more opaque. The picture holds premultiplied colours, which
/// a read-back turns into straight alpha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hologram {
    /// The colour of the shell as sRGB bytes.
    pub tint: [u8; 3],
    /// How opaque the body of the shell is, 0 to 1.
    pub opacity: f32,
    /// The strength of the edge glow, 0 to 4.
    pub glow: f32,
}

impl Default for Hologram {
    fn default() -> Self {
        Self {
            tint: [0x4d, 0xc7, 0xff],
            opacity: 0.25,
            glow: 1.0,
        }
    }
}

impl Hologram {
    fn sanitised(&self) -> (f32, f32) {
        let finite = |value: f32, fallback: f32| if value.is_finite() { value } else { fallback };
        let defaults = Self::default();
        (
            finite(self.opacity, defaults.opacity).clamp(0.0, 1.0),
            finite(self.glow, defaults.glow).clamp(0.0, MAX_GLOW),
        )
    }

    /// Returns the opacity the layers of the shell add up to at most, 0 to 1.
    pub(super) fn cap(&self) -> f32 {
        let (opacity, glow) = self.sanitised();
        EDGE_WEIGHT.mul_add(glow, opacity).clamp(0.0, 1.0)
    }
}

impl Shading {
    /// Returns whether the shading leaves the picture with premultiplied colours.
    #[must_use]
    pub const fn is_premultiplied(&self) -> bool {
        matches!(self, Self::Hologram(_))
    }

    /// Returns the two rows after the camera in the uniform buffer: the tint in linear colour
    /// with the share of the cap that the edges take, and the share that the body takes; zero for
    /// solid shading.
    pub(super) fn rows(&self) -> [[f32; 4]; 2] {
        match self {
            Self::Solid => [[0.0; 4]; 2],
            Self::Hologram(look) => {
                let [red, green, blue] = linear(look.tint);
                let (opacity, glow) = look.sanitised();
                let cap = look.cap();
                let share = |weight: f32| if cap > 0.0 { weight / cap } else { 0.0 };
                [
                    [red, green, blue, share(EDGE_WEIGHT * glow)],
                    [share(opacity), 0.0, 0.0, 0.0],
                ]
            }
        }
    }
}
