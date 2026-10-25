// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::model::{CUBE_CAPS_MATERIAL, TestSubmesh, TestVertex};
use crate::{TestModel, TestPositions};

impl TestModel {
    /// Returns a skin without nodes of `zones` triangles, 1 m apart along X, each its own submesh
    /// numbered by its position as the node, drawn with material ID [`CUBE_CAPS_MATERIAL`]: the
    /// shape of a body skin, whose submeshes are its zones.
    #[must_use]
    pub fn zoned(zones: usize) -> Self {
        let mut model = Self::cube()
            .with_positions(TestPositions::Float)
            .with_nodes(Vec::new());
        let at = |value: usize| u32::try_from(value).unwrap_or(u32::MAX);
        model.vertices = (0..zones)
            .flat_map(|zone| {
                let x = f32::from(u16::try_from(zone).unwrap_or(u16::MAX));
                [[x, 0.0], [x + 0.5, 0.0], [x, 0.5]]
                    .into_iter()
                    .map(|[px, pz]| TestVertex {
                        position: [px, 0.0, pz],
                        normal: [0.0, -1.0, 0.0],
                        uv: [0.0, 0.0],
                    })
            })
            .collect();
        model.indices = (0..at(zones.saturating_mul(3))).collect();
        model.submeshes = (0..zones)
            .map(|zone| TestSubmesh {
                material_id: CUBE_CAPS_MATERIAL,
                node: u16::try_from(zone).unwrap_or(u16::MAX),
                first_index: at(zone * 3),
                index_count: 3,
                first_vertex: at(zone * 3),
                vertex_count: 3,
            })
            .collect();
        model
    }
}
