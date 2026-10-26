// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::Mesh;

type Vec3 = [f32; 3];

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1].mul_add(b[2], -(a[2] * b[1])),
        a[2].mul_add(b[0], -(a[0] * b[2])),
        a[0].mul_add(b[1], -(a[1] * b[0])),
    ]
}

fn unit(vector: Vec3) -> Option<Vec3> {
    let length = vector[2]
        .mul_add(
            vector[2],
            vector[1].mul_add(vector[1], vector[0] * vector[0]),
        )
        .sqrt();
    (length.is_finite() && length > f32::EPSILON).then(|| vector.map(|value| value / length))
}

/// Returns the area-weighted normal of every vertex of the triangles of `mesh` placed at
/// `positions`.
fn surface_normals(mesh: &Mesh, positions: &[Vec3]) -> Vec<Vec3> {
    let mut sums = vec![[0.0_f32; 3]; positions.len()];
    let corner = |index: u32| positions.get(usize::try_from(index).ok()?).copied();
    for triangle in mesh.indices.as_chunks::<3>().0 {
        let [Some(a), Some(b), Some(c)] = triangle.map(corner) else {
            continue;
        };
        let normal = cross(sub(b, a), sub(c, a));
        for &index in triangle {
            if let Some(sum) = usize::try_from(index).ok().and_then(|at| sums.get_mut(at)) {
                for (total, value) in sum.iter_mut().zip(normal) {
                    *total += value;
                }
            }
        }
    }
    sums.into_iter()
        .map(|sum| unit(sum).unwrap_or_default())
        .collect()
}

fn dot(a: Vec3, b: Vec3) -> f32 {
    a[2].mul_add(b[2], a[1].mul_add(b[1], a[0] * b[0]))
}

/// Returns `vector` turned by the smallest rotation that takes the unit vector `from` to the
/// unit vector `to`, or `None` when the two point in opposite directions and no smallest rotation
/// exists.
fn turned_between(from: Vec3, to: Vec3, vector: Vec3) -> Option<Vec3> {
    let cosine = dot(from, to);
    if cosine < -0.999 {
        return None;
    }
    let axis = cross(from, to);
    let along = dot(axis, vector) / (1.0 + cosine);
    let sideways = cross(axis, vector);
    let mut turned = [0.0; 3];
    for (((out, axis), vector), side) in turned.iter_mut().zip(axis).zip(vector).zip(sideways) {
        *out = axis.mul_add(along, vector.mul_add(cosine, side));
    }
    Some(turned)
}

/// Returns the unit part of `tangent` perpendicular to the unit `normal` (Gram-Schmidt), or
/// `None` when the tangent is zero or lies along the normal, up to a relative residual of 1e-4.
fn orthogonalised(tangent: Vec3, normal: Vec3) -> Option<Vec3> {
    let along = dot(tangent, normal);
    let rest = [
        normal[0].mul_add(-along, tangent[0]),
        normal[1].mul_add(-along, tangent[1]),
        normal[2].mul_add(-along, tangent[2]),
    ];
    let size = dot(tangent, tangent).sqrt();
    (dot(rest, rest).sqrt() > size * 1e-4)
        .then(|| unit(rest))
        .flatten()
}

/// Returns a unit vector perpendicular to the unit `normal`.
fn perpendicular_to(normal: Vec3) -> Option<Vec3> {
    let [x, y, z] = normal.map(f32::abs);
    let axis = if x <= y && x <= z {
        [1.0, 0.0, 0.0]
    } else if y <= z {
        [0.0, 1.0, 0.0]
    } else {
        [0.0, 0.0, 1.0]
    };
    unit(cross(normal, axis))
}

/// Turns the tangent frame of `mesh` with its surface after its vertices moved from `before`:
/// each normal and tangent is rotated by the smallest rotation from the old surface normal at its
/// vertex to the new one, so the shading follows the new shape while seams and smoothing groups
/// of the authored frame stay as they were. The tangent is then made perpendicular to the normal
/// again and keeps its handedness `w`.
pub(super) fn carry_normals(mesh: &mut Mesh, before: &[Vec3]) {
    if mesh.normals.is_none() {
        return;
    }
    let old = surface_normals(mesh, before);
    let new = surface_normals(mesh, &mesh.positions);
    let Some(normals) = mesh.normals.as_mut() else {
        return;
    };
    for (index, (old, new)) in old.into_iter().zip(new).enumerate() {
        let (Some(normal), Some(old), Some(new)) = (normals.get_mut(index), unit(old), unit(new))
        else {
            continue;
        };
        let Some(turned) = turned_between(old, new, *normal).and_then(unit) else {
            continue;
        };
        *normal = turned;
        let Some(tangent) = mesh
            .tangents
            .as_mut()
            .and_then(|tangents| tangents.get_mut(index))
        else {
            continue;
        };
        let [x, y, z, w] = *tangent;
        let moved = turned_between(old, new, [x, y, z]).unwrap_or([x, y, z]);
        if let Some([x, y, z]) = orthogonalised(moved, turned).or_else(|| perpendicular_to(turned))
        {
            *tangent = [x, y, z, w];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{cross, dot, orthogonalised, perpendicular_to, turned_between};

    const EPSILON: f32 = 1e-5;

    fn near(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(b).all(|(a, b)| (a - b).abs() < EPSILON)
    }

    #[test]
    fn a_vector_turns_with_the_surface_normal() {
        let from = [0.0, 0.0, 1.0];
        let to = [0.0, 0.6, 0.8];

        assert!(near(
            turned_between(from, to, from).expect("a rotation"),
            to
        ));
        let turned = turned_between(from, to, [1.0, 0.0, 0.0]).expect("a rotation");
        assert!(near(turned, [1.0, 0.0, 0.0]), "the axis stays: {turned:?}");
        let sideways = turned_between(from, to, [0.0, 1.0, 0.0]).expect("a rotation");
        assert!(near(sideways, [0.0, 0.8, -0.6]), "{sideways:?}");
    }

    #[test]
    fn opposite_normals_have_no_smallest_rotation() {
        assert_eq!(
            turned_between([0.0, 0.0, 1.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]),
            None
        );
    }

    #[test]
    fn a_tangent_is_made_perpendicular_to_the_normal() {
        let normal = [0.0, 0.0, 1.0];
        let tangent = orthogonalised([1.0, 0.0, 0.5], normal).expect("a tangent");

        assert!(near(tangent, [1.0, 0.0, 0.0]), "{tangent:?}");
        assert!(dot(tangent, normal).abs() < EPSILON);
        assert!(near(cross(normal, tangent), [0.0, 1.0, 0.0]));
    }

    #[test]
    fn a_tangent_along_the_normal_has_no_perpendicular_part() {
        assert_eq!(orthogonalised([0.0, 0.0, 2.0], [0.0, 0.0, 1.0]), None);
        assert_eq!(orthogonalised([1e-6, 0.0, 1.0], [0.0, 0.0, 1.0]), None);
    }

    #[test]
    fn a_fallback_tangent_is_a_unit_vector_perpendicular_to_the_normal() {
        for normal in [[0.0, 0.0, 1.0], [0.6, 0.0, 0.8], [0.0, -1.0, 0.0]] {
            let tangent = perpendicular_to(normal).expect("a tangent");

            assert!(dot(tangent, normal).abs() < EPSILON, "{normal:?}");
            assert!((dot(tangent, tangent) - 1.0).abs() < EPSILON, "{normal:?}");
        }
    }
}
