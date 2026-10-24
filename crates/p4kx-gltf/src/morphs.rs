// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use gltf_json as json;
use json::accessor::sparse::{Indices, Sparse, Values};
use json::accessor::{ComponentType, GenericComponentType, IndexComponentType, Type};
use json::validation::Checked::Valid;
use json::validation::USize64;
use json::{Accessor, Index, Root};
use p4kx_model::Mesh;

use crate::axes::to_y_up;
use crate::block_data::BlockData;
use crate::layout::{Block, Layout, MeshLayout};

/// A morph target of a mesh: the vertices a shape change moves and by how much.
///
/// Written as a glTF morph target with a sparse `POSITION` accessor that holds only these
/// vertices, named by the mesh's `extras.targetNames`. The deltas are in the model's axes and
/// metres, like the mesh's positions, and are converted like them.
#[derive(Debug, Clone, PartialEq)]
pub struct MorphTarget {
    /// The name of the target, e.g. the expression it stands for.
    pub name: String,
    /// The moved vertices as indices into the mesh's vertices, strictly ascending.
    pub vertices: Vec<u32>,
    /// The move of each vertex in metres; one entry per vertex.
    pub deltas: Vec<[f32; 3]>,
}

impl MorphTarget {
    /// Returns whether the target moves at least one vertex of a mesh of `vertices` vertices, with
    /// one finite move per vertex and the vertices strictly ascending.
    fn fits(&self, vertices: usize) -> bool {
        self.vertices.len() == self.deltas.len()
            && self
                .vertices
                .last()
                .is_some_and(|&last| usize::try_from(last).is_ok_and(|last| last < vertices))
            && self.vertices.is_sorted_by(|a, b| a < b)
            && self.deltas.iter().flatten().all(|value| value.is_finite())
    }
}

/// The blocks of one morph target of a mesh and the bounds of its converted deltas.
#[derive(Debug, Clone)]
pub(crate) struct MorphLayout {
    pub(crate) name: String,
    pub(crate) indices: usize,
    pub(crate) deltas: usize,
    pub(crate) count: usize,
    pub(crate) min: [f32; 3],
    pub(crate) max: [f32; 3],
}

impl<'m> Layout<'m> {
    /// Plans the sparse blocks of the targets in `morphs` for the mesh planned last, which is
    /// `mesh`; targets that do not fit the mesh are left out.
    pub(crate) fn plan_morphs(&mut self, mesh: &'m Mesh, morphs: &'m [MorphTarget]) {
        let vertices = mesh.positions.len();
        let wide = u16::try_from(vertices).is_err();
        let mut planned = Vec::new();
        for morph in morphs.iter().filter(|morph| morph.fits(vertices)) {
            let rest = if morph.vertices.len() < vertices {
                0.0
            } else {
                f32::INFINITY
            };
            let (mut min, mut max) = ([rest; 3], [-rest; 3]);
            for &delta in &morph.deltas {
                for ((low, high), value) in min.iter_mut().zip(&mut max).zip(to_y_up(delta)) {
                    *low = low.min(value);
                    *high = high.max(value);
                }
            }
            planned.push(MorphLayout {
                name: morph.name.clone(),
                indices: self.push(BlockData::MorphIndices(&morph.vertices, wide)),
                deltas: self.push(BlockData::MorphDeltas(&morph.deltas)),
                count: morph.vertices.len(),
                min,
                max,
            });
        }
        if let Some(last) = self.meshes.last_mut() {
            last.morphs = planned;
        }
    }
}

/// Pushes the buffer views and the sparse `POSITION` accessor of each morph target of `mesh` and
/// returns the accessors in order.
pub(crate) fn push_targets(
    root: &mut Root,
    buffer: Index<json::Buffer>,
    layout: &Layout<'_>,
    mesh: &MeshLayout,
) -> Vec<Index<Accessor>> {
    let vertices = layout
        .blocks
        .get(mesh.positions)
        .map_or(0, |block| block.data.count());
    mesh.morphs
        .iter()
        .filter_map(|morph| {
            let indices = layout.blocks.get(morph.indices)?;
            let deltas = layout.blocks.get(morph.deltas)?;
            let component = match indices.data {
                BlockData::MorphIndices(_, true) => ComponentType::U32,
                _ => ComponentType::U16,
            };
            let bound = |values: [f32; 3]| {
                json::Value::from(values.into_iter().map(f64::from).collect::<Vec<_>>())
            };
            let indices = push_view(root, buffer, indices);
            let values = push_view(root, buffer, deltas);
            Some(root.push(Accessor {
                buffer_view: None,
                byte_offset: None,
                count: USize64(vertices as u64),
                component_type: Valid(GenericComponentType(ComponentType::F32)),
                type_: Valid(Type::Vec3),
                min: Some(bound(morph.min)),
                max: Some(bound(morph.max)),
                normalized: false,
                sparse: Some(Sparse {
                    count: USize64(morph.count as u64),
                    indices: Indices {
                        buffer_view: indices,
                        byte_offset: USize64(0),
                        component_type: Valid(IndexComponentType(component)),
                        extensions: None,
                        extras: json::extras::Void::default(),
                    },
                    values: Values {
                        buffer_view: values,
                        byte_offset: USize64(0),
                        extensions: None,
                        extras: json::extras::Void::default(),
                    },
                    extensions: None,
                    extras: json::extras::Void::default(),
                }),
                name: None,
                extensions: None,
                extras: json::extras::Void::default(),
            }))
        })
        .collect()
}

/// Returns the targets of a primitive: the delta accessors as `POSITION` targets, or `None`
/// without morph targets.
pub(crate) fn primitive_targets(
    accessors: &[Index<Accessor>],
) -> Option<Vec<json::mesh::MorphTarget>> {
    (!accessors.is_empty()).then(|| {
        accessors
            .iter()
            .map(|&positions| json::mesh::MorphTarget {
                positions: Some(positions),
                normals: None,
                tangents: None,
            })
            .collect()
    })
}

fn push_view(
    root: &mut Root,
    buffer: Index<json::Buffer>,
    block: &Block<'_>,
) -> Index<json::buffer::View> {
    root.push(json::buffer::View {
        buffer,
        byte_length: USize64(block.len),
        byte_offset: Some(USize64(block.offset)),
        byte_stride: None,
        target: None,
        name: None,
        extensions: None,
        extras: json::extras::Void::default(),
    })
}

/// Writes the JSON of `root`; the meshes with morph targets get `extras.targetNames`, which the
/// typed document cannot carry.
pub(crate) fn json_bytes(root: &Root, layout: &Layout<'_>) -> Result<Vec<u8>, serde_json::Error> {
    if layout.meshes.iter().all(|mesh| mesh.morphs.is_empty()) {
        return root.to_vec();
    }
    let mut value = root.to_value()?;
    if let Some(meshes) = value
        .get_mut("meshes")
        .and_then(serde_json::Value::as_array_mut)
    {
        for (mesh, layout) in meshes.iter_mut().zip(&layout.meshes) {
            if layout.morphs.is_empty() {
                continue;
            }
            let names: Vec<&str> = layout
                .morphs
                .iter()
                .map(|morph| morph.name.as_str())
                .collect();
            if let Some(object) = mesh.as_object_mut() {
                object.insert(
                    "extras".to_owned(),
                    serde_json::json!({ "targetNames": names }),
                );
            }
        }
    }
    serde_json::to_vec(&value)
}
