// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use gltf_json::extensions::scene::khr_lights_punctual::{Light, Spot, Type};
use gltf_json::validation::Checked::Valid;
use p4kx_model::Transform;

const MAX_CONE: f32 = std::f32::consts::FRAC_PI_2;
const MIN_CONE: f32 = 1.0e-4;

/// Turns the frame a light is placed in, in which it shines along +X as in `CryEngine`, into the
/// frame glTF reads, in which it shines along the node's -Z, which is `CryEngine`'s +Y.
pub(crate) const LIGHT_FRAME: Transform = Transform {
    rows: [
        [0.0, 1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ],
};

/// The shape of a [`SceneLight`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum SceneLightKind {
    /// A point light that shines in all directions from the node.
    Point,
    /// A spot light that shines in a cone along the node's +X axis, in radians. The writer keeps
    /// the outer angle within 0 to a quarter turn and the inner angle below it.
    Spot {
        /// The angle from the axis where the falloff begins.
        inner_cone: f32,
        /// The angle from the axis where the light ends.
        outer_cone: f32,
    },
    /// A directional light that shines along the node's +X axis.
    Directional,
}

/// A punctual light at a [`SceneNode`](crate::SceneNode), written with `KHR_lights_punctual`.
///
/// It shines along the +X axis of the node's `CryEngine` space, which is how the engine orients
/// lights; the writer turns the node so that glTF's -Z axis lies along it.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct SceneLight {
    /// The shape of the light.
    pub kind: SceneLightKind,
    /// The colour in linear red, green and blue, each in 0 to 1; written clamped.
    pub colour: [f32; 3],
    /// The intensity, written as given, or zero when negative or not finite. glTF reads it as
    /// candela for point and spot lights and lux for directional lights; a caller that passes the
    /// raw engine value, as the Interior does, makes it relative and approximate.
    pub intensity: f32,
    /// The distance in metres beyond which the light has no effect; left out when not positive
    /// or not finite.
    pub range: Option<f32>,
}

impl SceneLight {
    /// Creates a light of `kind` with `colour` and `intensity` and no range.
    #[must_use]
    pub const fn new(kind: SceneLightKind, colour: [f32; 3], intensity: f32) -> Self {
        Self {
            kind,
            colour,
            intensity,
            range: None,
        }
    }

    /// Returns the light with the range `range`.
    #[must_use]
    pub const fn with_range(mut self, range: f32) -> Self {
        self.range = Some(range);
        self
    }

    pub(crate) fn to_json(self, name: &str) -> Light {
        let clean = |value: f32| if value.is_finite() { value } else { 0.0 };
        let (type_, spot) = match self.kind {
            SceneLightKind::Point => (Type::Point, None),
            SceneLightKind::Directional => (Type::Directional, None),
            SceneLightKind::Spot {
                inner_cone,
                outer_cone,
            } => {
                let outer = clean(outer_cone).clamp(MIN_CONE, MAX_CONE);
                let inner = clean(inner_cone).clamp(0.0, outer - MIN_CONE / 2.0);
                (
                    Type::Spot,
                    Some(Spot {
                        inner_cone_angle: inner,
                        outer_cone_angle: outer,
                    }),
                )
            }
        };
        Light {
            color: self.colour.map(|channel| clean(channel).clamp(0.0, 1.0)),
            extensions: None,
            extras: gltf_json::extras::Void::default(),
            intensity: clean(self.intensity).max(0.0),
            name: (!name.is_empty()).then(|| name.to_owned()),
            range: self.range.filter(|range| range.is_finite() && *range > 0.0),
            spot,
            type_: Valid(type_),
        }
    }
}
