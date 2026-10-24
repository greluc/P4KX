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

/// Turns the normals of `mesh` with its surface after its vertices moved from `before`: each
/// normal gets the change of the surface normal at its vertex, so the shading follows the new
/// shape while seams and smoothing groups of the authored normals stay as they were.
pub(super) fn carry_normals(mesh: &mut Mesh, before: &[Vec3]) {
    if mesh.normals.is_none() {
        return;
    }
    let old = surface_normals(mesh, before);
    let new = surface_normals(mesh, &mesh.positions);
    let Some(normals) = mesh.normals.as_mut() else {
        return;
    };
    for ((normal, old), new) in normals.iter_mut().zip(old).zip(new) {
        let turned = [
            normal[0] + new[0] - old[0],
            normal[1] + new[1] - old[1],
            normal[2] + new[2] - old[2],
        ];
        if let Some(turned) = unit(turned) {
            *normal = turned;
        }
    }
}
