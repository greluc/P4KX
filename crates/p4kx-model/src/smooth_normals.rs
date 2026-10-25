// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::vector::{cross, normalise};

/// Computes smooth unit vertex normals: the area-weighted mean of the face normals of the
/// triangles around each vertex. A triangle index outside `positions` is ignored, and a vertex
/// no triangle uses, or whose faces cancel, gets +Z.
pub(crate) fn smooth_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut sums = vec![[0.0_f32; 3]; positions.len()];
    for triangle in indices.as_chunks::<3>().0 {
        let corners =
            triangle.map(|index| positions.get(usize::try_from(index).unwrap_or(usize::MAX)));
        let [Some(&a), Some(&b), Some(&c)] = corners else {
            continue;
        };
        let face = cross(sub(b, a), sub(c, a));
        for index in triangle {
            if let Some(sum) = sums.get_mut(usize::try_from(*index).unwrap_or(usize::MAX)) {
                for (total, part) in sum.iter_mut().zip(face) {
                    *total += part;
                }
            }
        }
    }
    sums.into_iter().map(normalise).collect()
}

fn sub([ax, ay, az]: [f32; 3], [bx, by, bz]: [f32; 3]) -> [f32; 3] {
    [ax - bx, ay - by, az - bz]
}

#[cfg(test)]
mod tests {
    use super::smooth_normals;

    #[test]
    fn a_triangle_in_the_xy_plane_faces_up() {
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = smooth_normals(&positions, &[0, 1, 2]);
        assert_eq!(normals, vec![[0.0, 0.0, 1.0]; 3]);
    }

    #[test]
    fn a_shared_vertex_averages_the_faces_by_area() {
        let positions = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let normals = smooth_normals(&positions, &[0, 1, 2, 0, 3, 1]);
        let [x, y, z] = normals[0];
        assert!(x.abs() < 1e-6, "{x}");
        assert!((y - z).abs() < 1e-6 && z > 0.0, "{y} {z}");
    }

    #[test]
    fn indices_outside_the_vertices_are_ignored() {
        let positions = [[0.0; 3]; 2];
        assert_eq!(smooth_normals(&positions, &[0, 1, 9]).len(), 2);
    }
}
