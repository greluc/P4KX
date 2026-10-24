// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;

use p4kx_extraction::ModelSource;
use p4kx_extraction::model::{
    Bone, BoneTransform, ChunkUsage, Mesh, Model, ModelChunk, ModelFile, Node, Submesh,
};
use serde::Serialize;

use p4kx_extraction::AnimationSet;

use crate::model_animations::{AnimationJson, UnreadJson};
use crate::model_materials_json::{LibraryJson, MaterialJson};

/// The JSON shape of `p4kx model info`.
#[derive(Serialize)]
pub(crate) struct InfoJson<'a> {
    model: &'a str,
    geometry: Option<&'a str>,
    material_library: Option<&'a str>,
    material_ids: Vec<u16>,
    library: LibraryJson<'a>,
    materials: Vec<MaterialJson<'a>>,
    nodes: Vec<NodeJson<'a>>,
    lods: Vec<LodJson>,
    bones: Vec<BoneJson<'a>>,
    animations: Vec<AnimationJson>,
    unread_animations: Vec<UnreadJson>,
    chunks: Vec<ChunkJson>,
}

impl<'a> InfoJson<'a> {
    pub(crate) fn of(source: &'a ModelSource, animations: &AnimationSet) -> Self {
        let model = source.model();
        Self {
            model: source.name(),
            geometry: model.geometry.as_deref(),
            material_library: model.material_library.as_deref(),
            material_ids: model.material_ids(),
            library: LibraryJson::of(model),
            materials: MaterialJson::of_model(model),
            nodes: model.nodes.iter().enumerate().map(NodeJson::of).collect(),
            lods: model
                .lods
                .iter()
                .map(|lod| LodJson {
                    level: lod.level,
                    vertices: lod.vertex_count(),
                    triangles: lod.triangle_count(),
                    meshes: lod.meshes.iter().map(MeshJson::of).collect(),
                })
                .collect(),
            bones: model.bones.iter().enumerate().map(BoneJson::of).collect(),
            animations: AnimationJson::of_set(model, animations),
            unread_animations: animations.unread.iter().map(UnreadJson::of).collect(),
            chunks: model.chunks.iter().map(ChunkJson::of).collect(),
        }
    }
}

#[derive(Serialize)]
struct NodeJson<'a> {
    index: usize,
    name: &'a str,
    parent: Option<usize>,
    transform: [[f32; 4]; 3],
}

impl<'a> NodeJson<'a> {
    fn of((index, node): (usize, &'a Node)) -> Self {
        Self {
            index,
            name: &node.name,
            parent: node.parent,
            transform: node.transform.rows,
        }
    }
}

#[derive(Serialize)]
struct LodJson {
    level: u32,
    vertices: usize,
    triangles: usize,
    meshes: Vec<MeshJson>,
}

#[derive(Serialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the JSON has one flag per optional vertex stream of a mesh"
)]
struct MeshJson {
    node: usize,
    vertices: usize,
    triangles: usize,
    normals: bool,
    tangents: bool,
    colours: bool,
    skinned: bool,
    submeshes: Vec<SubmeshJson>,
}

impl MeshJson {
    fn of(mesh: &Mesh) -> Self {
        Self {
            node: mesh.node,
            vertices: mesh.positions.len(),
            triangles: mesh.indices.len() / 3,
            normals: mesh.normals.is_some(),
            tangents: mesh.tangents.is_some(),
            colours: mesh.colours.is_some(),
            skinned: mesh.influences.is_some(),
            submeshes: mesh.submeshes.iter().map(SubmeshJson::of).collect(),
        }
    }
}

#[derive(Serialize)]
struct SubmeshJson {
    material_id: u16,
    first_index: usize,
    index_count: usize,
}

impl SubmeshJson {
    const fn of(submesh: &Submesh) -> Self {
        Self {
            material_id: submesh.material_id,
            first_index: submesh.first_index,
            index_count: submesh.index_count,
        }
    }
}

#[derive(Serialize)]
struct BoneJson<'a> {
    index: usize,
    name: &'a str,
    parent: Option<usize>,
    controller_id: u32,
    node: Option<usize>,
    relative: PoseJson,
    world: PoseJson,
}

impl<'a> BoneJson<'a> {
    fn of((index, bone): (usize, &'a Bone)) -> Self {
        Self {
            index,
            name: &bone.name,
            parent: bone.parent,
            controller_id: bone.controller_id,
            node: bone.node,
            relative: PoseJson::of(&bone.relative),
            world: PoseJson::of(&bone.world),
        }
    }
}

#[derive(Serialize)]
struct PoseJson {
    rotation: [f32; 4],
    translation: [f32; 3],
}

impl PoseJson {
    const fn of(pose: &BoneTransform) -> Self {
        Self {
            rotation: pose.rotation,
            translation: pose.translation,
        }
    }
}

/// The JSON shape of one chunk of a model's files.
#[derive(Serialize)]
pub(crate) struct ChunkJson {
    file: Cow<'static, str>,
    index: usize,
    #[serde(rename = "type")]
    type_name: Option<&'static str>,
    type_id: u32,
    usage: &'static str,
}

impl ChunkJson {
    pub(crate) fn of(chunk: &ModelChunk) -> Self {
        Self {
            file: file_name(chunk.file),
            index: chunk.index,
            type_name: chunk.chunk_type.name(),
            type_id: chunk.chunk_type.id(),
            usage: usage_name(chunk.usage),
        }
    }
}

/// Returns `model`, `geometry` or `lod<n>-geometry`.
pub(crate) fn file_name(file: ModelFile) -> Cow<'static, str> {
    match file {
        ModelFile::Model => Cow::Borrowed("model"),
        ModelFile::Geometry => Cow::Borrowed("geometry"),
        ModelFile::LodGeometry(level) => Cow::Owned(format!("lod{level}-geometry")),
        _ => Cow::Borrowed("other"),
    }
}

/// Returns `read`, `skipped` or `unknown`.
pub(crate) const fn usage_name(usage: ChunkUsage) -> &'static str {
    match usage {
        ChunkUsage::Read => "read",
        ChunkUsage::Skipped => "skipped",
        ChunkUsage::Unknown => "unknown",
    }
}

/// Returns the name of the node `index`, or `-` when it has none.
pub(crate) fn node_name(model: &Model, index: usize) -> &str {
    model
        .nodes
        .get(index)
        .map(|node| node.name.as_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("-")
}
