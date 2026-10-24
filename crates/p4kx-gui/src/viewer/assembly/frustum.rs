// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::super::camera::Matrix;

/// The six planes of a view volume and the scale that turns a sphere into pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Frustum {
    planes: [[f32; 4]; 6],
    depth_row: [f32; 4],
    pixels_per_unit: f32,
}

fn row(matrix: &Matrix, at: usize) -> [f32; 4] {
    std::array::from_fn(|column| {
        matrix
            .get(column)
            .and_then(|values| values.get(at))
            .copied()
            .unwrap_or(0.0)
    })
}

fn combine(first: [f32; 4], second: [f32; 4], sign: f32) -> [f32; 4] {
    std::array::from_fn(|at| {
        let a = first.get(at).copied().unwrap_or(0.0);
        let b = second.get(at).copied().unwrap_or(0.0);
        sign.mul_add(b, a)
    })
}

fn normalised(plane: [f32; 4]) -> [f32; 4] {
    let length = plane[2]
        .mul_add(plane[2], plane[1].mul_add(plane[1], plane[0] * plane[0]))
        .sqrt();
    if length > 0.0 {
        plane.map(|value| value / length)
    } else {
        plane
    }
}

impl Frustum {
    /// Extracts the planes of `view_proj`, the column-major world to clip matrix with depth from
    /// 0 to 1, for a view `height` pixels high.
    pub(super) fn of(view_proj: &Matrix, height: f32) -> Self {
        let (x, y, z, w) = (
            row(view_proj, 0),
            row(view_proj, 1),
            row(view_proj, 2),
            row(view_proj, 3),
        );
        let planes = [
            combine(w, x, 1.0),
            combine(w, x, -1.0),
            combine(w, y, 1.0),
            combine(w, y, -1.0),
            z,
            combine(w, z, -1.0),
        ]
        .map(normalised);
        let focal = y[2].mul_add(y[2], y[1].mul_add(y[1], y[0] * y[0])).sqrt();
        Self {
            planes,
            depth_row: w,
            pixels_per_unit: 0.5 * focal * height.max(1.0),
        }
    }

    /// Returns whether the sphere may touch the view volume.
    pub(super) fn sees(&self, centre: [f32; 3], radius: f32) -> bool {
        self.planes.iter().all(|plane| {
            plane[0].mul_add(
                centre[0],
                plane[1].mul_add(centre[1], plane[2].mul_add(centre[2], plane[3])),
            ) >= -radius
        })
    }

    /// Returns the radius in pixels the sphere covers on screen, 0 when it is behind the eye.
    pub(super) fn pixels(&self, centre: [f32; 3], radius: f32) -> f32 {
        let depth = self.depth_row[0].mul_add(
            centre[0],
            self.depth_row[1].mul_add(
                centre[1],
                self.depth_row[2].mul_add(centre[2], self.depth_row[3]),
            ),
        );
        if depth > radius {
            radius * self.pixels_per_unit / depth
        } else {
            f32::INFINITY
        }
    }
}
