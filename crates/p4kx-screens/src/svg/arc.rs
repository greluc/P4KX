// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::f32::consts::{FRAC_PI_2, TAU};

type Pt = (f32, f32);

/// The parameters of an SVG elliptical arc besides its end points.
pub(super) struct ArcShape {
    pub(super) radii: Pt,
    pub(super) turn_degrees: f32,
    pub(super) large: bool,
    pub(super) sweep: bool,
}

/// An arc as a centre, a start angle and a sweep, from which cubic pieces are cut.
pub(super) struct Arc {
    centre: Pt,
    radii: Pt,
    turn: (f32, f32),
    start: f32,
    sweep: f32,
}

fn dot(a: f32, b: f32, c: f32, d: f32) -> f32 {
    a.mul_add(b, c * d)
}

impl Arc {
    /// Converts the end point form of SVG to the centre form; `None` when a radius is zero.
    pub(super) fn of(from: Pt, to: Pt, shape: &ArcShape) -> Option<Self> {
        let (mut rx, mut ry) = (shape.radii.0.abs(), shape.radii.1.abs());
        if rx < f32::EPSILON || ry < f32::EPSILON {
            return None;
        }
        let (sin, cos) = shape.turn_degrees.to_radians().sin_cos();
        let (half_x, half_y) = ((from.0 - to.0) / 2.0, (from.1 - to.1) / 2.0);
        let (x1, y1) = (
            dot(cos, half_x, sin, half_y),
            dot(cos, half_y, -sin, half_x),
        );
        let excess = dot(x1, x1, y1, y1 * rx * rx / (ry * ry)) / (rx * rx);
        if excess > 1.0 {
            let root = excess.sqrt();
            rx *= root;
            ry *= root;
        }
        let (rx2, ry2) = (rx * rx, ry * ry);
        let numerator = (ry2 * x1).mul_add(-x1, dot(rx2, ry2, -rx2 * y1, y1));
        let denominator = dot(rx2 * y1, y1, ry2 * x1, x1);
        let direction = if shape.large == shape.sweep {
            -1.0
        } else {
            1.0
        };
        let factor = direction * (numerator / denominator).max(0.0).sqrt();
        let (cx1, cy1) = (factor * rx * y1 / ry, -factor * ry * x1 / rx);
        let centre = (
            dot(cos, cx1, -sin, cy1) + f32::midpoint(from.0, to.0),
            dot(sin, cx1, cos, cy1) + f32::midpoint(from.1, to.1),
        );
        let start = ((y1 - cy1) / ry).atan2((x1 - cx1) / rx);
        let end = ((-y1 - cy1) / ry).atan2((-x1 - cx1) / rx);
        let mut sweep = end - start;
        if shape.sweep && sweep < 0.0 {
            sweep += TAU;
        } else if !shape.sweep && sweep > 0.0 {
            sweep -= TAU;
        }
        Some(Self {
            centre,
            radii: (rx, ry),
            turn: (sin, cos),
            start,
            sweep,
        })
    }

    /// Returns how many cubic pieces of at most a quarter turn the arc is cut into.
    pub(super) fn pieces(&self) -> usize {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "at most four pieces make a full turn"
        )]
        let count = (self.sweep.abs() / FRAC_PI_2).ceil().max(1.0) as usize;
        count
    }

    fn at(&self, angle: f32) -> Pt {
        let (sin, cos) = self.turn;
        let (s, c) = angle.sin_cos();
        let (x, y) = (self.radii.0 * c, self.radii.1 * s);
        (
            dot(cos, x, -sin, y) + self.centre.0,
            dot(sin, x, cos, y) + self.centre.1,
        )
    }

    fn slope(&self, angle: f32) -> Pt {
        let (sin, cos) = self.turn;
        let (s, c) = angle.sin_cos();
        let (x, y) = (-self.radii.0 * s, self.radii.1 * c);
        (dot(cos, x, -sin, y), dot(sin, x, cos, y))
    }

    /// Returns the control points and the end point of each cubic piece.
    pub(super) fn cubics(&self) -> impl Iterator<Item = (Pt, Pt, Pt)> + '_ {
        #[expect(
            clippy::cast_precision_loss,
            reason = "the piece count is at most four"
        )]
        let delta = self.sweep / self.pieces() as f32;
        let reach = 4.0 / 3.0 * (delta / 4.0).tan();
        (0..self.pieces()).map(move |piece| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "the piece index is at most three"
            )]
            let theta = (piece as f32).mul_add(delta, self.start);
            let next = theta + delta;
            let (p0, p3) = (self.at(theta), self.at(next));
            let (d0, d1) = (self.slope(theta), self.slope(next));
            (
                (d0.0.mul_add(reach, p0.0), d0.1.mul_add(reach, p0.1)),
                (d1.0.mul_add(-reach, p3.0), d1.1.mul_add(-reach, p3.1)),
                p3,
            )
        })
    }
}
