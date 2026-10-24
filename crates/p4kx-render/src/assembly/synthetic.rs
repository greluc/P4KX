// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A small ship of boxes for the tests that draw: the nose points along +X, Z is up.

use super::{AssemblyScene, Group, Item, Left, PartDraw, PartInfo};
use crate::scene::{Bounds, Rows, Vertex, next_serial};

const FACES: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
    ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
    ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
    ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
    ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ([0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
];

struct Box {
    size: [f32; 3],
    at: [f32; 3],
    turn: f32,
}

const BOXES: [Box; 7] = [
    Box {
        size: [7.0, 1.6, 1.2],
        at: [0.0, 0.0, 0.0],
        turn: 0.0,
    },
    Box {
        size: [2.2, 1.0, 0.7],
        at: [1.6, 0.0, 0.85],
        turn: 0.0,
    },
    Box {
        size: [3.4, 4.2, 0.18],
        at: [-0.6, 2.9, -0.1],
        turn: -0.45,
    },
    Box {
        size: [3.4, 4.2, 0.18],
        at: [-0.6, -2.9, -0.1],
        turn: 0.45,
    },
    Box {
        size: [1.6, 0.18, 1.7],
        at: [-3.0, 0.0, 1.3],
        turn: 0.0,
    },
    Box {
        size: [1.4, 0.7, 0.7],
        at: [-3.6, 1.0, 0.0],
        turn: 0.0,
    },
    Box {
        size: [1.4, 0.7, 0.7],
        at: [-3.6, -1.0, 0.0],
        turn: 0.0,
    },
];

fn cube() -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for (normal, right, up) in FACES {
        let base = u32::try_from(vertices.len()).unwrap_or_default();
        for (u, v) in [(-1.0_f32, -1.0_f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let at: [f32; 3] = std::array::from_fn(|axis| {
                0.5 * v.mul_add(up[axis], u.mul_add(right[axis], normal[axis]))
            });
            vertices.push([
                at[0],
                at[1],
                at[2],
                normal[0],
                normal[1],
                normal[2],
                0.5_f32.mul_add(u, 0.5),
                0.5_f32.mul_add(v, 0.5),
            ]);
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (vertices, indices)
}

fn rows(part: &Box) -> Rows {
    let (sin, cos) = part.turn.sin_cos();
    let [sx, sy, sz] = part.size;
    let [x, y, z] = part.at;
    [
        [cos * sx, -sin * sy, 0.0, x],
        [sin * sx, cos * sy, 0.0, y],
        [0.0, 0.0, sz, z],
    ]
}

/// Returns the ship: seven boxes sharing one cube mesh, a hull, a canopy, two wings, a fin and
/// two engines.
pub(crate) fn ship() -> AssemblyScene {
    let (vertices, indices) = cube();
    let mut bounds = Bounds::EMPTY;
    let mut draws = Vec::new();
    let mut parts = Vec::new();
    let mut items = Vec::new();
    for (at, part) in BOXES.iter().enumerate() {
        let world = rows(part);
        let radius = 0.5 * part.size.iter().map(|side| side * side).sum::<f32>().sqrt();
        let centre = part.at;
        bounds.include(centre.map(|value| value - radius));
        bounds.include(centre.map(|value| value + radius));
        let number = u32::try_from(at).unwrap_or_default();
        items.push(Item {
            group: 0,
            colour: [0.55, 0.60, 0.65, 1.0],
        });
        draws.push(PartDraw {
            part: at,
            world,
            centre,
            radius,
            items: number..number + 1,
        });
        parts.push(PartInfo {
            name: format!("port{at}/part"),
            path: format!("port{at}"),
            parent: at.checked_sub(1).map(|_| 0),
            triangles: 12,
            interior: false,
        });
    }
    AssemblyScene {
        vertices,
        indices,
        groups: vec![Group {
            first_index: 0,
            index_count: 36,
            texture: None,
            translucent: false,
        }],
        items,
        draws,
        parts,
        textures: Vec::new(),
        meshes: 1,
        bounds,
        left: Left::default(),
        serial: next_serial(),
    }
}
