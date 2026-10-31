// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Where the vertices of one mesh lie on the surface of another.

use rayon::prelude::*;

pub(super) type Vec3 = [f32; 3];

/// The corners of each edge of a triangle, as indices into its points.
const EDGES: [[usize; 2]; 3] = [[0, 1], [1, 2], [2, 0]];

/// A point on a surface: the corners of a triangle and the barycentric weights of the point in it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Anchor {
    pub(super) corners: [u32; 3],
    pub(super) weights: [f32; 3],
}

/// A triangle of the surface with the bounding sphere that lets a search skip it.
struct Triangle {
    corners: [u32; 3],
    points: [Vec3; 3],
    centre: Vec3,
    radius: f32,
}

fn sub(from: Vec3, by: Vec3) -> Vec3 {
    [from[0] - by[0], from[1] - by[1], from[2] - by[2]]
}

fn dot(left: Vec3, right: Vec3) -> f32 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

fn distance_squared(first: Vec3, second: Vec3) -> f32 {
    let delta = sub(first, second);
    dot(delta, delta)
}

/// Returns the sum of `points` weighted by `weights`.
fn blend(points: &[Vec3; 3], weights: [f32; 3]) -> Vec3 {
    let mut sum = [0.0_f32; 3];
    for (point, weight) in points.iter().zip(weights) {
        for (total, value) in sum.iter_mut().zip(point) {
            *total = weight.mul_add(*value, *total);
        }
    }
    sum
}

impl Triangle {
    fn new(corners: [u32; 3], positions: &[Vec3]) -> Option<Self> {
        let [first, second, third] = corners;
        let points = [
            *positions.get(first as usize)?,
            *positions.get(second as usize)?,
            *positions.get(third as usize)?,
        ];
        let centre = blend(&points, [1.0 / 3.0; 3]);
        let radius = points
            .iter()
            .map(|&point| distance_squared(point, centre))
            .fold(0.0_f32, f32::max)
            .sqrt();
        Some(Self {
            corners,
            points,
            centre,
            radius,
        })
    }

    /// Returns the weights of the point of the triangle nearest to `point`.
    fn nearest_weights(&self, point: Vec3) -> [f32; 3] {
        if let Some(inside) = self.inside_weights(point) {
            return inside;
        }
        EDGES
            .iter()
            .map(|&[from, to]| self.on_edge(from, to, point))
            .map(|weights| {
                (
                    distance_squared(point, blend(&self.points, weights)),
                    weights,
                )
            })
            .min_by(|left, right| left.0.total_cmp(&right.0))
            .map_or([1.0, 0.0, 0.0], |(_, weights)| weights)
    }

    /// Returns the barycentric weights of `point` projected onto the plane of the triangle, or
    /// `None` when the projection lies outside it or the triangle is degenerate.
    fn inside_weights(&self, point: Vec3) -> Option<[f32; 3]> {
        let [origin, second, third] = self.points;
        let (edge_b, edge_c, offset) =
            (sub(second, origin), sub(third, origin), sub(point, origin));
        let (bb, bc, cc) = (
            dot(edge_b, edge_b),
            dot(edge_b, edge_c),
            dot(edge_c, edge_c),
        );
        let (pb, pc) = (dot(offset, edge_b), dot(offset, edge_c));
        let denominator = bb.mul_add(cc, -(bc * bc));
        if denominator.is_nan() || denominator <= 0.0 {
            return None;
        }
        let weight_b = cc.mul_add(pb, -(bc * pc)) / denominator;
        let weight_c = bb.mul_add(pc, -(bc * pb)) / denominator;
        let weights = [1.0 - weight_b - weight_c, weight_b, weight_c];
        weights
            .iter()
            .all(|weight| *weight >= 0.0)
            .then_some(weights)
    }

    /// Returns the weights of the point of the edge from corner `from` to corner `to` nearest to
    /// `point`.
    fn on_edge(&self, from: usize, to: usize, point: Vec3) -> [f32; 3] {
        let mut weights = [0.0_f32; 3];
        let (Some(&start), Some(&end)) = (self.points.get(from), self.points.get(to)) else {
            return weights;
        };
        let along = sub(end, start);
        let length = dot(along, along);
        let reach = if length > 0.0 {
            (dot(sub(point, start), along) / length).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if let Some(weight) = weights.get_mut(from) {
            *weight = 1.0 - reach;
        }
        if let Some(weight) = weights.get_mut(to) {
            *weight = reach;
        }
        weights
    }
}

/// Returns the point of `triangles` nearest to `point`, or `None` for no usable triangle.
fn nearest(triangles: &[Triangle], point: Vec3) -> Option<Anchor> {
    let mut best: Option<(f32, Anchor)> = None;
    for triangle in triangles {
        if let Some((found, _)) = best {
            let reach = distance_squared(point, triangle.centre).sqrt() - triangle.radius;
            if reach > 0.0 && reach * reach >= found {
                continue;
            }
        }
        let weights = triangle.nearest_weights(point);
        let squared = distance_squared(point, blend(&triangle.points, weights));
        if squared.is_finite() && best.is_none_or(|(found, _)| squared < found) {
            best = Some((
                squared,
                Anchor {
                    corners: triangle.corners,
                    weights,
                },
            ));
        }
    }
    best.map(|(_, anchor)| anchor)
}

/// Returns, for each of `points`, the point of the surface (`positions` and the triangle list
/// `indices`) nearest to it. A triangle with an index beyond `positions` is left out; a point
/// without any usable triangle has no anchor.
pub(super) fn anchors(positions: &[Vec3], indices: &[u32], points: &[Vec3]) -> Vec<Option<Anchor>> {
    let triangles: Vec<Triangle> = indices
        .as_chunks::<3>()
        .0
        .iter()
        .filter_map(|&corners| Triangle::new(corners, positions))
        .collect();
    points
        .par_iter()
        .map(|&point| nearest(&triangles, point))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Anchor, anchors};

    const CORNERS: [[f32; 3]; 3] = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];

    fn weights_of(point: [f32; 3]) -> [f32; 3] {
        let found = anchors(&CORNERS, &[0, 1, 2], &[point]);
        let Some(Some(Anchor { weights, .. })) = found.first().copied() else {
            panic!("no anchor");
        };
        weights
    }

    fn close(found: [f32; 3], wanted: [f32; 3]) {
        for (have, want) in found.iter().zip(wanted) {
            assert!((have - want).abs() < 1e-6, "{found:?} against {wanted:?}");
        }
    }

    #[test]
    fn a_point_above_the_triangle_has_its_barycentric_weights() {
        close(weights_of([0.25, 0.5, 3.0]), [0.25, 0.25, 0.5]);
    }

    #[test]
    fn a_point_beyond_a_corner_takes_the_corner() {
        close(weights_of([-1.0, -1.0, 0.0]), [1.0, 0.0, 0.0]);
        close(weights_of([2.0, -1.0, 0.0]), [0.0, 1.0, 0.0]);
        close(weights_of([-1.0, 2.0, 0.0]), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn a_point_beyond_an_edge_takes_the_point_on_the_edge() {
        close(weights_of([0.5, -1.0, 0.0]), [0.5, 0.5, 0.0]);
        close(weights_of([1.0, 1.0, 0.0]), [0.0, 0.5, 0.5]);
    }

    #[test]
    fn the_nearest_of_two_triangles_wins() {
        let positions = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 5.0],
            [1.0, 0.0, 5.0],
            [0.0, 1.0, 5.0],
        ];
        let found = anchors(&positions, &[0, 1, 2, 3, 4, 5], &[[0.2, 0.2, 4.0]]);
        assert_eq!(
            found
                .first()
                .copied()
                .flatten()
                .map(|anchor| anchor.corners),
            Some([3, 4, 5])
        );
    }

    #[test]
    fn a_surface_without_usable_triangles_gives_no_anchor() {
        assert_eq!(anchors(&CORNERS, &[0, 1, 9], &[[0.0; 3]]), vec![None]);
    }
}
