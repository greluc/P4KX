// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::DataForge;

use crate::read::{float, narrow};

const CONFIG: &str = "UIHoloVehicle_Config";
const DEFAULT_FOV: f32 = 10.0;
const DEFAULT_VIEW_ANGLE: f32 = 60.0;
const DEFAULT_DISTANCE_SCALER: f32 = 1.5;

/// The camera of the vehicle hologram the game's `UIHoloVehicle_Config` Record sets: the field
/// of view, the angle the vehicle is turned in and how far the camera stands from it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HologramCamera {
    fov_degrees: f32,
    view_angle_degrees: f32,
    distance_scaler: f32,
}

impl Default for HologramCamera {
    /// Returns the values the Record has in the current game: a field of view of 10 degrees, a
    /// view angle of 60 degrees and a distance scaler of 1.5.
    fn default() -> Self {
        Self {
            fov_degrees: DEFAULT_FOV,
            view_angle_degrees: DEFAULT_VIEW_ANGLE,
            distance_scaler: DEFAULT_DISTANCE_SCALER,
        }
    }
}

impl HologramCamera {
    /// Returns the camera of a field of view in degrees, a view angle in degrees and a distance
    /// scaler.
    #[must_use]
    pub const fn new(fov_degrees: f32, view_angle_degrees: f32, distance_scaler: f32) -> Self {
        Self {
            fov_degrees,
            view_angle_degrees,
            distance_scaler,
        }
    }

    /// Reads the camera from the `UIHoloVehicle_Config` Record of `forge`, a field it lacks
    /// keeping the value of [`HologramCamera::default`], or returns `None` when the DataForge
    /// has no such Record; with several the one with the smallest name is read.
    #[must_use]
    pub fn read(forge: &DataForge<'_>) -> Option<Self> {
        let record = forge
            .records()
            .filter(|record| record.type_name().is_ok_and(|name| name == CONFIG))
            .min_by_key(|record| record.name().unwrap_or_default())?;
        let tree = record.tree().ok()?;
        let root = tree.root();
        let default = Self::default();
        let field = |name: &str, fallback: f32| float(root, name).map_or(fallback, narrow);
        Some(Self {
            fov_degrees: field("cameraFOV", default.fov_degrees),
            view_angle_degrees: field("ownDefaultViewAngle", default.view_angle_degrees),
            distance_scaler: field("ownCameraDistanceScaler", default.distance_scaler),
        })
    }

    /// Returns the vertical field of view in degrees.
    #[must_use]
    pub const fn fov_degrees(&self) -> f32 {
        self.fov_degrees
    }

    /// Returns the angle in degrees the vehicle is turned in, about its vertical axis.
    #[must_use]
    pub const fn view_angle_degrees(&self) -> f32 {
        self.view_angle_degrees
    }

    /// Returns the factor on the distance at which the vehicle just fills the view.
    #[must_use]
    pub const fn distance_scaler(&self) -> f32 {
        self.distance_scaler
    }
}
