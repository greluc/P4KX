// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gfx::Affine;

use crate::geometry::Geometry;
use crate::paint::Area;

const SINGULAR: f32 = 1.0e-6;

/// What a widget's turn and scale do to the box it was laid out in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Placement {
    /// Laid out as it is.
    Plain,
    /// Moved, turned or scaled about its pivot.
    Moved(Affine),
    /// Flattened to a line or a point: nothing of it is visible.
    Flat,
}

/// Returns the linear part of the orthographic projection of the turn of `degrees` about the
/// x, y and z axes, applied in that order.
fn projected(degrees: [f32; 3]) -> [[f32; 2]; 2] {
    let [x, y, z] = degrees.map(f32::to_radians);
    let (sx, cx) = x.sin_cos();
    let (sy, cy) = y.sin_cos();
    let (sz, cz) = z.sin_cos();
    [
        [cz * cy, cz.mul_add(sy * sx, -(sz * cx))],
        [sz * cy, sz.mul_add(sy * sx, cz * cx)],
    ]
}

fn is_zero(angles: [f32; 3]) -> bool {
    angles.iter().all(|angle| angle.abs() < f32::EPSILON)
}

fn is_unit(scale: [f32; 2]) -> bool {
    scale
        .iter()
        .all(|factor| (factor - 1.0).abs() < f32::EPSILON)
}

impl Placement {
    /// Returns the placement of a widget laid out in `area`: the turn about the x, y and z axes in
    /// degrees, then the scale, both about the pivot point of the box, in pixels of the layout.
    pub(crate) fn of(area: Area, geometry: &Geometry) -> Self {
        if is_unit(geometry.scale) && is_zero(geometry.rotation) {
            return Self::Plain;
        }
        let turn = projected(geometry.rotation);
        let [sx, sy] = geometry.scale;
        let a = turn[0][0] * sx;
        let b = turn[1][0] * sx;
        let c = turn[0][1] * sy;
        let d = turn[1][1] * sy;
        if b.mul_add(-c, a * d).abs() < SINGULAR {
            return Self::Flat;
        }
        let px = geometry.pivot[0].mul_add(area.width, area.x);
        let py = geometry.pivot[1].mul_add(area.height, area.y);
        let moved = Affine {
            a,
            b,
            c,
            d,
            tx: px - a.mul_add(px, c * py),
            ty: py - b.mul_add(px, d * py),
        };
        if moved.is_finite() {
            Self::Moved(moved)
        } else {
            Self::Flat
        }
    }
}
