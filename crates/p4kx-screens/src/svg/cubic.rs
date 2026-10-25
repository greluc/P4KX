// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

type Pt = (f32, f32);

const PIECES: u8 = 4;

/// A cubic Bézier curve, cut into quadratic pieces for the scene.
pub(super) struct Cubic {
    points: [Pt; 4],
}

fn weigh(values: [f32; 4], weights: [f32; 4]) -> f32 {
    values
        .into_iter()
        .zip(weights)
        .fold(0.0, |sum, (value, weight)| value.mul_add(weight, sum))
}

impl Cubic {
    pub(super) const fn new(from: Pt, c1: Pt, c2: Pt, to: Pt) -> Self {
        Self {
            points: [from, c1, c2, to],
        }
    }

    fn component(&self, pick: fn(&Pt) -> f32) -> [f32; 4] {
        self.points.map(|point| pick(&point))
    }

    fn at(&self, t: f32) -> Pt {
        let u = 1.0 - t;
        let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
        (
            weigh(self.component(|p| p.0), weights),
            weigh(self.component(|p| p.1), weights),
        )
    }

    fn slope(&self, t: f32) -> Pt {
        let u = 1.0 - t;
        let weights = [
            -3.0 * u * u,
            3.0 * u * 2.0f32.mul_add(-t, u),
            3.0 * t * 2.0f32.mul_add(u, -t),
            3.0 * t * t,
        ];
        (
            weigh(self.component(|p| p.0), weights),
            weigh(self.component(|p| p.1), weights),
        )
    }

    /// Returns the control point and the end point of each quadratic piece.
    pub(super) fn quads(&self) -> impl Iterator<Item = (Pt, Pt)> + '_ {
        let step = 1.0 / f32::from(PIECES);
        (0..PIECES).map(move |piece| {
            let (t0, t1) = (f32::from(piece) * step, f32::from(piece + 1) * step);
            let (p0, p3) = (self.at(t0), self.at(t1));
            let (d0, d1) = (self.slope(t0), self.slope(t1));
            let reach = (t1 - t0) / 3.0;
            let a = (d0.0.mul_add(reach, p0.0), d0.1.mul_add(reach, p0.1));
            let b = (d1.0.mul_add(-reach, p3.0), d1.1.mul_add(-reach, p3.1));
            let control = (
                3.0f32.mul_add(a.0 + b.0, -(p0.0 + p3.0)) / 4.0,
                3.0f32.mul_add(a.1 + b.1, -(p0.1 + p3.1)) / 4.0,
            );
            (
                control,
                if piece + 1 == PIECES {
                    self.points[3]
                } else {
                    p3
                },
            )
        })
    }
}
