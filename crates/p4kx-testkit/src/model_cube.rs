// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::model::{
    CUBE_CAPS_MATERIAL, CUBE_MESH_OFFSET, CUBE_SIDES_MATERIAL, TestNode, TestSubmesh, TestVertex,
};

const HALF_EDGE: f32 = 2.0;
const CENTRE: [f32; 3] = [0.0, 0.0, 2.0];
const IDENTITY: [[f32; 4]; 3] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

const FACES: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
    ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ([0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
    ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
    ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
    ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
    ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
];

const CORNERS: [(f32, f32, [f32; 2]); 4] = [
    (-1.0, -1.0, [0.0, 0.0]),
    (1.0, -1.0, [1.0, 0.0]),
    (1.0, 1.0, [1.0, 1.0]),
    (-1.0, 1.0, [0.0, 1.0]),
];

pub(crate) fn cube_vertices() -> Vec<TestVertex> {
    FACES
        .iter()
        .flat_map(|&(normal, u, v)| {
            CORNERS.iter().map(move |&(a, b, uv)| {
                let position = std::array::from_fn(|axis| {
                    let at = |vector: [f32; 3]| vector.get(axis).copied().unwrap_or(0.0);
                    HALF_EDGE.mul_add(b.mul_add(at(v), a.mul_add(at(u), at(normal))), at(CENTRE))
                });
                TestVertex {
                    position,
                    normal,
                    uv,
                }
            })
        })
        .collect()
}

pub(crate) fn cube_nodes() -> Vec<TestNode> {
    let [x, y, z] = CUBE_MESH_OFFSET;
    vec![
        TestNode {
            name: "cube".to_owned(),
            parent: None,
            local: IDENTITY,
            kind: 2,
        },
        TestNode {
            name: "cube_mesh".to_owned(),
            parent: Some(0),
            local: [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]],
            kind: 0,
        },
    ]
}

pub(crate) fn cube_submeshes() -> Vec<TestSubmesh> {
    vec![
        TestSubmesh {
            material_id: CUBE_CAPS_MATERIAL,
            node: 1,
            first_index: 0,
            index_count: 12,
            first_vertex: 0,
            vertex_count: 8,
        },
        TestSubmesh {
            material_id: CUBE_SIDES_MATERIAL,
            node: 1,
            first_index: 12,
            index_count: 24,
            first_vertex: 8,
            vertex_count: 16,
        },
    ]
}
