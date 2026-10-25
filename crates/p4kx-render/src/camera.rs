// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use super::scene::Bounds;

const FOV_Y: f32 = FRAC_PI_4;
const HOME_YAW: f32 = -2.4;
const HOME_PITCH: f32 = 0.45;
const MAX_PITCH: f32 = FRAC_PI_2 - 0.01;
const FIT_MARGIN: f32 = 1.15;
const MIN_DISTANCE: f32 = 0.02;
const MAX_DISTANCE: f32 = 60.0;
const RADIANS_PER_PIXEL: f32 = 0.008;
const ZOOM_PER_STEP: f32 = 0.9;
const KEY_ORBIT_PIXELS: f32 = 24.0;

/// A matrix in the column-major layout of the GPU.
pub type Matrix = [[f32; 4]; 4];

/// What the shader reads once per frame; 96 bytes in the layout of the uniform buffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Uniforms {
    /// World to clip space.
    pub view_proj: Matrix,
    /// The position of the eye; `w` is the distance of the near plane.
    pub eye: [f32; 4],
    /// The direction towards the key light; `w` is the distance of the far plane.
    pub key_light: [f32; 4],
}

/// A step of the keyboard control, all of them made by the arrow keys with or without a modifier
/// and the plus, minus and reset keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Orbit one key step to the left.
    Left,
    /// Orbit one key step to the right.
    Right,
    /// Orbit one key step up.
    Up,
    /// Orbit one key step down.
    Down,
}

impl Step {
    const fn pixels(self) -> (f32, f32) {
        match self {
            Self::Left => (-KEY_ORBIT_PIXELS, 0.0),
            Self::Right => (KEY_ORBIT_PIXELS, 0.0),
            Self::Up => (0.0, KEY_ORBIT_PIXELS),
            Self::Down => (0.0, -KEY_ORBIT_PIXELS),
        }
    }
}

/// How the vehicle hologram looks at its subject: through a narrow lens from a fixed side, at a
/// distance that leaves room around the vehicle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HologramView {
    /// The vertical field of view in degrees; clamped to 1 to 120.
    pub fov_degrees: f32,
    /// The angle in degrees, in the horizontal plane, between the nose (+Y) and the direction
    /// the camera looks from; positive towards +X.
    pub view_angle_degrees: f32,
    /// The height of the camera above the horizontal plane, as an angle in degrees; clamped to
    /// -80 to 80.
    pub elevation_degrees: f32,
    /// The factor on the distance at which the bounding sphere of the vehicle just fits;
    /// clamped to 0.5 to 10.
    pub distance_scaler: f32,
}

impl HologramView {
    /// Returns the view with every value that is not finite replaced by that of
    /// [`HologramView::default`] and the others clamped to their ranges, as the camera uses them.
    #[must_use]
    pub fn sanitised(&self) -> Self {
        let defaults = Self::default();
        let value = |wanted: f32, fallback: f32| if wanted.is_finite() { wanted } else { fallback };
        Self {
            fov_degrees: value(self.fov_degrees, defaults.fov_degrees).clamp(1.0, 120.0),
            view_angle_degrees: value(self.view_angle_degrees, defaults.view_angle_degrees)
                .rem_euclid(360.0),
            elevation_degrees: value(self.elevation_degrees, defaults.elevation_degrees)
                .clamp(-80.0, 80.0),
            distance_scaler: value(self.distance_scaler, defaults.distance_scaler).clamp(0.5, 10.0),
        }
    }
}

impl Default for HologramView {
    /// Returns the values of the `UIHoloVehicle_Config` Record of the current game and an
    /// elevation of 20 degrees.
    fn default() -> Self {
        Self {
            fov_degrees: 10.0,
            view_angle_degrees: 60.0,
            elevation_degrees: 20.0,
            distance_scaler: 1.5,
        }
    }
}

/// An orbit camera around a target with Z up, framing a [`Bounds`].
///
/// Dragging orbits, a secondary drag pans, the wheel and the plus and minus keys zoom, and
/// [`Camera::reset`] frames the model again.
#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    target: [f32; 3],
    yaw: f32,
    pitch: f32,
    distance: f32,
    centre: [f32; 3],
    radius: f32,
    fitted: bool,
    fitted_aspect: f32,
    fov: f32,
    margin: f32,
    home: (f32, f32),
}

impl Camera {
    /// Frames `bounds` from the home direction; the distance is fitted to the view once the
    /// first frame knows its aspect, see [`Camera::fit`].
    #[must_use]
    pub fn framing(bounds: &Bounds) -> Self {
        let radius = bounds.radius();
        Self {
            target: bounds.centre(),
            yaw: HOME_YAW,
            pitch: HOME_PITCH,
            distance: fit_distance(radius, 1.0),
            centre: bounds.centre(),
            radius,
            fitted: false,
            fitted_aspect: 1.0,
            fov: FOV_Y,
            margin: FIT_MARGIN,
            home: (HOME_YAW, HOME_PITCH),
        }
    }

    /// Frames `bounds` the way the vehicle hologram does: through the lens and from the side that
    /// `view` names, at the distance that fits the bounding sphere times the distance scaler.
    /// Values that are not finite are replaced by those of [`HologramView::default`].
    #[must_use]
    pub fn holographic(bounds: &Bounds, view: &HologramView) -> Self {
        let view = view.sanitised();
        let mut camera = Self::framing(bounds);
        camera.fov = view.fov_degrees.to_radians();
        camera.margin = view.distance_scaler;
        camera.yaw =
            (FRAC_PI_2 - view.view_angle_degrees.to_radians()).rem_euclid(std::f32::consts::TAU);
        camera.pitch = view.elevation_degrees.to_radians();
        camera.home = (camera.yaw, camera.pitch);
        camera.distance = fit_distance_with(camera.radius, 1.0, (camera.fov, camera.margin));
        camera
    }

    /// Returns whether [`Camera::fit`] was called since the camera was made.
    #[must_use]
    pub const fn is_fitted(&self) -> bool {
        self.fitted
    }

    /// Frames the whole model in a view of `aspect` (width over height).
    pub fn fit(&mut self, aspect: f32) {
        self.distance = fit_distance_with(self.radius, aspect, (self.fov, self.margin));
        self.fitted_aspect = aspect;
        self.fitted = true;
    }

    /// Returns to the home direction and frames the whole model in a view of `aspect`.
    pub fn reset(&mut self, aspect: f32) {
        self.target = self.centre;
        (self.yaw, self.pitch) = self.home;
        self.fit(aspect);
    }

    /// Returns whether the camera looks at the model from the home direction and the distance it
    /// was last fitted to; resizing the view does not move it away from home.
    #[must_use]
    pub fn is_home(&self) -> bool {
        let near = |a: f32, b: f32| (a - b).abs() < 1.0e-4 * self.radius.max(1.0);
        self.target
            .iter()
            .zip(self.centre)
            .all(|(a, b)| near(*a, b))
            && (self.yaw - self.home.0).abs() < 1.0e-5
            && (self.pitch - self.home.1).abs() < 1.0e-5
            && near(
                self.distance,
                fit_distance_with(self.radius, self.fitted_aspect, (self.fov, self.margin)),
            )
    }

    /// Turns the camera around the target by a drag of `dx` and `dy` pixels.
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw = (-dx)
            .mul_add(RADIANS_PER_PIXEL, self.yaw)
            .rem_euclid(std::f32::consts::TAU);
        self.pitch = dy
            .mul_add(RADIANS_PER_PIXEL, self.pitch)
            .clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Orbits by one step of the keyboard.
    pub fn step(&mut self, step: Step) {
        let (dx, dy) = step.pixels();
        self.orbit(dx, dy);
    }

    /// Pans by one step of the keyboard in a view `height` points high.
    pub fn pan_step(&mut self, step: Step, height: f32) {
        let (dx, dy) = step.pixels();
        self.pan(dx, dy, height);
    }

    /// Moves the target across the view by a drag of `dx` and `dy` pixels in a view `height`
    /// pixels high, so the model follows the pointer.
    pub fn pan(&mut self, dx: f32, dy: f32, height: f32) {
        let per_pixel = 2.0 * self.distance * (0.5 * self.fov).tan() / height.max(1.0);
        let (right, up, _) = self.basis();
        for axis in 0..3 {
            if let (Some(target), Some(right), Some(up)) =
                (self.target.get_mut(axis), right.get(axis), up.get(axis))
            {
                *target = (-dx).mul_add(*right, dy * *up).mul_add(per_pixel, *target);
            }
        }
    }

    /// Moves the camera closer for positive `steps` and away for negative ones.
    pub fn zoom(&mut self, steps: f32) {
        let factor = ZOOM_PER_STEP.powf(steps);
        self.distance =
            (self.distance * factor).clamp(MIN_DISTANCE * self.radius, MAX_DISTANCE * self.radius);
    }

    /// Returns the position of the eye.
    #[must_use]
    pub fn eye(&self) -> [f32; 3] {
        let (_, _, back) = self.basis();
        std::array::from_fn(|axis| {
            back.get(axis)
                .copied()
                .unwrap_or(0.0)
                .mul_add(self.distance, self.target.get(axis).copied().unwrap_or(0.0))
        })
    }

    /// Returns the distance from the eye to the target.
    #[must_use]
    pub const fn distance(&self) -> f32 {
        self.distance
    }

    /// Returns the point the camera looks at.
    #[must_use]
    pub const fn target(&self) -> [f32; 3] {
        self.target
    }

    /// Returns what the shader needs for a view of `aspect`.
    #[must_use]
    pub fn uniforms(&self, aspect: f32) -> Uniforms {
        let (right, up, back) = self.basis();
        let eye = self.eye();
        let offset = length(sub(self.target, self.centre));
        let near = (self.distance - self.radius - offset).max(self.radius * 0.005);
        let far = (self.distance + self.radius + offset).max(near * 2.0);
        let view = look_at(eye, right, up, back);
        let projection = perspective(aspect, near, far, self.fov);
        let key = normalise(add(
            add(scale(right, -0.5), scale(up, 0.7)),
            scale(back, 0.5),
        ));
        Uniforms {
            view_proj: multiply(&projection, &view),
            eye: [eye[0], eye[1], eye[2], near],
            key_light: [key[0], key[1], key[2], far],
        }
    }

    fn basis(&self) -> ([f32; 3], [f32; 3], [f32; 3]) {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let back = [cos_pitch * cos_yaw, cos_pitch * sin_yaw, sin_pitch];
        let right = [-sin_yaw, cos_yaw, 0.0];
        let up = cross(back, right);
        (right, up, back)
    }
}

fn fit_distance(radius: f32, aspect: f32) -> f32 {
    fit_distance_with(radius, aspect, (FOV_Y, FIT_MARGIN))
}

fn fit_distance_with(radius: f32, aspect: f32, (fov, margin): (f32, f32)) -> f32 {
    let half_y = 0.5 * fov;
    let half_x = (half_y.tan() * aspect.max(0.05)).atan();
    radius * margin / half_y.min(half_x).sin()
}

fn look_at(eye: [f32; 3], right: [f32; 3], up: [f32; 3], back: [f32; 3]) -> Matrix {
    [
        [right[0], up[0], back[0], 0.0],
        [right[1], up[1], back[1], 0.0],
        [right[2], up[2], back[2], 0.0],
        [-dot(right, eye), -dot(up, eye), -dot(back, eye), 1.0],
    ]
}

fn perspective(aspect: f32, near: f32, far: f32, fov: f32) -> Matrix {
    let focal = 1.0 / (0.5 * fov).tan();
    let depth = far / (near - far);
    [
        [focal / aspect.max(0.05), 0.0, 0.0, 0.0],
        [0.0, focal, 0.0, 0.0],
        [0.0, 0.0, depth, -1.0],
        [0.0, 0.0, near * depth, 0.0],
    ]
}

fn multiply(left: &Matrix, right: &Matrix) -> Matrix {
    std::array::from_fn(|column| {
        std::array::from_fn(|row| {
            (0..4)
                .map(|k| {
                    left.get(k).and_then(|c| c.get(row)).copied().unwrap_or(0.0)
                        * right
                            .get(column)
                            .and_then(|c| c.get(k))
                            .copied()
                            .unwrap_or(0.0)
                })
                .sum()
        })
    })
}

pub(super) fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0].mul_add(b[0], a[1].mul_add(b[1], a[2] * b[2]))
}

pub(super) fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1].mul_add(b[2], -(a[2] * b[1])),
        a[2].mul_add(b[0], -(a[0] * b[2])),
        a[0].mul_add(b[1], -(a[1] * b[0])),
    ]
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f32; 3], factor: f32) -> [f32; 3] {
    a.map(|value| value * factor)
}

fn length(a: [f32; 3]) -> f32 {
    dot(a, a).sqrt()
}

fn normalise(a: [f32; 3]) -> [f32; 3] {
    scale(a, 1.0 / length(a).max(f32::MIN_POSITIVE))
}

#[cfg(test)]
mod tests;
