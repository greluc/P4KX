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

    /// Returns the light with the values the GLB writer writes: the colour within 0 to 1, the
    /// intensity not negative, the range only when positive, and the cones of a spot light within
    /// 0 to a quarter turn with the inner one below the outer one. Values that are not finite
    /// become zero.
    #[must_use]
    pub fn clamped(self) -> Self {
        let clean = |value: f32| if value.is_finite() { value } else { 0.0 };
        let kind = match self.kind {
            SceneLightKind::Spot {
                inner_cone,
                outer_cone,
            } => {
                let outer = clean(outer_cone).clamp(MIN_CONE, MAX_CONE);
                let inner = clean(inner_cone).clamp(0.0, outer - MIN_CONE / 2.0);
                SceneLightKind::Spot {
                    inner_cone: inner,
                    outer_cone: outer,
                }
            }
            other => other,
        };
        Self {
            kind,
            colour: self.colour.map(|channel| clean(channel).clamp(0.0, 1.0)),
            intensity: clean(self.intensity).max(0.0),
            range: self.range.filter(|range| range.is_finite() && *range > 0.0),
        }
    }

    pub(crate) fn to_json(self, name: &str) -> Light {
        let clamped = self.clamped();
        let (type_, spot) = match clamped.kind {
            SceneLightKind::Point => (Type::Point, None),
            SceneLightKind::Directional => (Type::Directional, None),
            SceneLightKind::Spot {
                inner_cone,
                outer_cone,
            } => (
                Type::Spot,
                Some(Spot {
                    inner_cone_angle: inner_cone,
                    outer_cone_angle: outer_cone,
                }),
            ),
        };
        Light {
            color: clamped.colour,
            extensions: None,
            extras: gltf_json::extras::Extras::default(),
            intensity: clamped.intensity,
            name: (!name.is_empty()).then(|| name.to_owned()),
            range: clamped.range,
            spot,
            type_: Valid(type_),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SceneLight, SceneLightKind};

    #[test]
    fn clamped_keeps_what_the_writer_writes() {
        let light = SceneLight::new(
            SceneLightKind::Spot {
                inner_cone: 2.0,
                outer_cone: 3.0,
            },
            [2.0, -1.0, f32::NAN],
            -4.0,
        )
        .with_range(0.0)
        .clamped();

        assert_eq!(light.colour, [1.0, 0.0, 0.0]);
        assert!(light.intensity.abs() < f32::EPSILON);
        assert_eq!(light.range, None);
        let SceneLightKind::Spot {
            inner_cone,
            outer_cone,
        } = light.kind
        else {
            panic!("a spot light stays a spot light");
        };
        assert!((outer_cone - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert!(inner_cone < outer_cone);
    }
}
