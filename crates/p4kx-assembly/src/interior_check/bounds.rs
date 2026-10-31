// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{Model, Transform};

/// An axis-aligned box in world space, in metres, built from finite points only.
///
/// A box may be degenerate: a single point or a flat or thin slab has zero extent on some axes
/// and still contains its own points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldBox {
    min: [f32; 3],
    max: [f32; 3],
}

fn finite(point: [f32; 3]) -> bool {
    point.iter().all(|value| value.is_finite())
}

/// Returns `point` moved by `transform`.
pub(crate) fn transform_point(transform: &Transform, [x, y, z]: [f32; 3]) -> [f32; 3] {
    transform
        .rows
        .map(|[a, b, c, d]| a.mul_add(x, b.mul_add(y, c.mul_add(z, d))))
}

impl WorldBox {
    /// Returns the box of a single point, or `None` when a coordinate is not finite.
    #[must_use]
    pub fn point(point: [f32; 3]) -> Option<Self> {
        finite(point).then_some(Self {
            min: point,
            max: point,
        })
    }

    /// Returns the smallest box around the finite points of `points`, or `None` when there is
    /// none; points with a NaN or infinite coordinate are skipped.
    #[must_use]
    pub fn around(points: impl IntoIterator<Item = [f32; 3]>) -> Option<Self> {
        let mut bounds = None;
        for point in points {
            Self::grow(&mut bounds, point);
        }
        bounds
    }

    /// Grows `bounds` to hold `point`, starting it when it is `None`; a point that is not finite
    /// is skipped.
    pub fn grow(bounds: &mut Option<Self>, point: [f32; 3]) {
        if !finite(point) {
            return;
        }
        match bounds {
            Some(existing) => {
                for ((low, high), value) in existing
                    .min
                    .iter_mut()
                    .zip(existing.max.iter_mut())
                    .zip(point)
                {
                    *low = low.min(value);
                    *high = high.max(value);
                }
            }
            None => *bounds = Self::point(point),
        }
    }

    /// Grows `bounds` to hold the whole of `other`, starting it when it is `None`.
    pub fn grow_with_box(bounds: &mut Option<Self>, other: &Self) {
        Self::grow(bounds, other.min);
        Self::grow(bounds, other.max);
    }

    /// Returns the volume in cubic metres; a degenerate box has none.
    #[must_use]
    pub fn volume(&self) -> f32 {
        self.min
            .iter()
            .zip(&self.max)
            .map(|(low, high)| high - low)
            .product()
    }

    /// Returns how far in metres this box protrudes beyond `outer` on its worst axis, 0 when it
    /// lies inside.
    #[must_use]
    pub fn overshoot(&self, outer: &Self) -> f32 {
        self.min
            .iter()
            .zip(&self.max)
            .zip(outer.min.iter().zip(&outer.max))
            .map(|((low, high), (outer_low, outer_high))| {
                (outer_low - low).max(high - outer_high).max(0.0)
            })
            .fold(0.0, f32::max)
    }

    /// Grows `bounds` around the first level of detail of `model`, placed by `placement` (the
    /// transform from the model's space to the world): each mesh's vertices carried through its
    /// node chain. A model without meshes adds nothing.
    pub fn grow_with_model(bounds: &mut Option<Self>, model: &Model, placement: &Transform) {
        let Some(level) = model.lods.first() else {
            return;
        };
        for mesh in &level.meshes {
            let Some(chain) = node_chain(model, mesh.node) else {
                continue;
            };
            let world = placement.then(&chain);
            for position in &mesh.positions {
                Self::grow(bounds, transform_point(&world, *position));
            }
        }
    }

    /// Returns the smallest corner.
    #[must_use]
    pub const fn min(&self) -> [f32; 3] {
        self.min
    }

    /// Returns the largest corner.
    #[must_use]
    pub const fn max(&self) -> [f32; 3] {
        self.max
    }

    /// Returns whether `point` lies inside the box or on its surface; a point that is not finite
    /// lies in no box.
    #[must_use]
    pub fn contains(&self, point: [f32; 3]) -> bool {
        self.distance(point) == Some(0.0)
    }

    /// Returns the distance from `point` to the box in metres, 0 for a point inside or on the
    /// surface, or `None` when the point is not finite or the distance overflows.
    #[must_use]
    pub fn distance(&self, point: [f32; 3]) -> Option<f32> {
        if !finite(point) {
            return None;
        }
        let squared: f32 = point
            .iter()
            .zip(self.min.iter().zip(&self.max))
            .map(|(value, (low, high))| {
                let outside = (low - value).max(value - high).max(0.0);
                outside * outside
            })
            .sum();
        let distance = squared.sqrt();
        distance.is_finite().then_some(distance)
    }
}

/// Returns the transform from the space of node `index` of `model` to the model's space, or
/// `None` when the node is missing or the parents loop.
fn node_chain(model: &Model, index: usize) -> Option<Transform> {
    let mut chain = Transform::IDENTITY;
    let mut at = Some(index);
    for _ in 0..=model.nodes.len() {
        let Some(current) = at else {
            return Some(chain);
        };
        let node = model.nodes.get(current)?;
        chain = node.transform.then(&chain);
        at = node.parent;
    }
    None
}
