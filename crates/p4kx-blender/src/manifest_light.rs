// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gltf::{SceneLight, SceneLightKind};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The shape of a [`Light`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LightKind {
    /// Shines in all directions from the node.
    Point,
    /// Shines in a cone along the node's -Z axis.
    Spot,
    /// Shines along the node's -Z axis from infinitely far away.
    Directional,
}

/// A punctual light at a node of the kind `light`.
///
/// The node's transform is already turned so that the light shines along its local -Z axis, as
/// glTF and Blender lights do.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Light {
    /// The shape of the light.
    pub kind: LightKind,
    /// The colour in linear red, green and blue, each in 0 to 1.
    pub colour: [f32; 3],
    /// The intensity as the engine states it, not converted to a photometric unit; the engine's
    /// unit is not documented, so the value is only good for comparing lights of one package.
    pub intensity: f32,
    /// The distance in metres beyond which the light has no effect; absent when the light has
    /// none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<f32>,
    /// The angle in radians from the axis where a spot light's falloff begins; present for the
    /// kind `spot`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inner_cone: Option<f32>,
    /// The angle in radians from the axis where a spot light ends, within 0 to a quarter turn;
    /// present for the kind `spot`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outer_cone: Option<f32>,
}

impl Light {
    /// Returns the light as the GLB writer would write `light`.
    pub(crate) fn of(light: SceneLight) -> Self {
        let light = light.clamped();
        let (kind, inner_cone, outer_cone) = match light.kind {
            SceneLightKind::Spot {
                inner_cone,
                outer_cone,
            } => (LightKind::Spot, Some(inner_cone), Some(outer_cone)),
            SceneLightKind::Directional => (LightKind::Directional, None, None),
            _ => (LightKind::Point, None, None),
        };
        Self {
            kind,
            colour: light.colour,
            intensity: light.intensity,
            range: light.range,
            inner_cone,
            outer_cone,
        }
    }
}
