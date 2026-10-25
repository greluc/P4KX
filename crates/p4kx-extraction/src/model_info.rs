// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;

use p4kx_model::{
    Bone, BoneTransform, ChunkUsage, Mesh, Model, ModelChunk, ModelFile, Node, Submesh,
};
use serde::Serialize;

use crate::model_info_animations::{AnimationInfo, UnreadInfo};
use crate::model_info_materials::{LibraryInfo, MaterialInfo};
use crate::{AnimationSet, ModelSource};

/// What `p4kx model info --json` says about a model, and what front-ends serialise of it.
#[derive(Debug, Serialize)]
pub struct ModelInfo<'a> {
    /// The Entry path or file of the model.
    pub model: &'a str,
    /// The geometry companion the model names.
    pub geometry: Option<&'a str>,
    /// The material library the model names.
    pub material_library: Option<&'a str>,
    /// The material IDs the meshes use.
    pub material_ids: Vec<u16>,
    /// What reading the material library came to.
    pub library: LibraryInfo<'a>,
    /// The materials of the library.
    pub materials: Vec<MaterialInfo<'a>>,
    /// The list of material indices of the node table.
    pub node_material_indices: &'a [u16],
    /// What a lenient read worked around, as sentences.
    pub issues: Vec<String>,
    /// The nodes of the model.
    pub nodes: Vec<NodeInfo<'a>>,
    /// The levels of detail with their meshes.
    pub lods: Vec<LodInfo>,
    /// The bones of the skeleton.
    pub bones: Vec<BoneInfo<'a>>,
    /// The animations read for the model.
    pub animations: Vec<AnimationInfo>,
    /// The animation files and animations that could not be read.
    pub unread_animations: Vec<UnreadInfo>,
    /// Every chunk of the model's files.
    pub chunks: Vec<ChunkInfo>,
}

impl<'a> ModelInfo<'a> {
    /// Describes `source` and the `animations` read for it.
    #[must_use]
    pub fn of(source: &'a ModelSource, animations: &AnimationSet) -> Self {
        let model = source.model();
        Self {
            model: source.name(),
            geometry: model.geometry.as_deref(),
            material_library: model.material_library.as_deref(),
            material_ids: model.material_ids(),
            library: LibraryInfo::of(model),
            materials: MaterialInfo::of_model(model),
            node_material_indices: &model.material_indices,
            issues: model.issues.iter().map(ToString::to_string).collect(),
            nodes: model.nodes.iter().enumerate().map(NodeInfo::of).collect(),
            lods: model
                .lods
                .iter()
                .map(|lod| LodInfo {
                    level: lod.level,
                    vertices: lod.vertex_count(),
                    triangles: lod.triangle_count(),
                    meshes: lod.meshes.iter().map(MeshInfo::of).collect(),
                })
                .collect(),
            bones: model.bones.iter().enumerate().map(BoneInfo::of).collect(),
            animations: AnimationInfo::of_set(model, animations),
            unread_animations: animations.unread.iter().map(UnreadInfo::of).collect(),
            chunks: model.chunks.iter().map(ChunkInfo::of).collect(),
        }
    }
}

/// One node of a model.
#[derive(Debug, Serialize)]
pub struct NodeInfo<'a> {
    index: usize,
    name: &'a str,
    parent: Option<usize>,
    transform: [[f32; 4]; 3],
    scale: [f32; 3],
    geometry_type: Option<u16>,
    helper: bool,
    properties: Vec<PropertyInfo<'a>>,
}

#[derive(Debug, Serialize)]
struct PropertyInfo<'a> {
    key: &'a str,
    value: &'a str,
}

impl<'a> NodeInfo<'a> {
    fn of((index, node): (usize, &'a Node)) -> Self {
        Self {
            index,
            name: &node.name,
            parent: node.parent,
            transform: node.transform.rows,
            scale: node.scale,
            geometry_type: node.geometry_type,
            helper: node.is_helper(),
            properties: node
                .properties
                .iter()
                .map(|(key, value)| PropertyInfo { key, value })
                .collect(),
        }
    }
}

/// One level of detail of a model.
#[derive(Debug, Serialize)]
pub struct LodInfo {
    /// The level of detail, 0 for the finest.
    pub level: u32,
    /// The vertices of all meshes of the level.
    pub vertices: usize,
    /// The triangles of all meshes of the level.
    pub triangles: usize,
    /// The meshes of the level.
    pub meshes: Vec<MeshInfo>,
}

/// One mesh of a level of detail.
#[derive(Debug, Serialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the JSON has one flag per optional vertex stream of a mesh"
)]
pub struct MeshInfo {
    node: usize,
    vertices: usize,
    triangles: usize,
    normals: bool,
    normals_computed: bool,
    tangents: bool,
    colours: bool,
    second_uvs: bool,
    skinned: bool,
    submeshes: Vec<SubmeshInfo>,
}

impl MeshInfo {
    fn of(mesh: &Mesh) -> Self {
        Self {
            node: mesh.node,
            vertices: mesh.positions.len(),
            triangles: mesh.indices.len() / 3,
            normals: mesh.normals.is_some(),
            normals_computed: mesh.normals_computed,
            tangents: mesh.tangents.is_some(),
            colours: mesh.colours.is_some(),
            second_uvs: mesh.second_uvs.is_some(),
            skinned: mesh.influences.is_some(),
            submeshes: mesh.submeshes.iter().map(SubmeshInfo::of).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
struct SubmeshInfo {
    material_id: u16,
    first_index: usize,
    index_count: usize,
}

impl SubmeshInfo {
    const fn of(submesh: &Submesh) -> Self {
        Self {
            material_id: submesh.material_id,
            first_index: submesh.first_index,
            index_count: submesh.index_count,
        }
    }
}

/// One bone of a model's skeleton.
#[derive(Debug, Serialize)]
pub struct BoneInfo<'a> {
    index: usize,
    name: &'a str,
    parent: Option<usize>,
    controller_id: u32,
    node: Option<usize>,
    relative: PoseInfo,
    world: PoseInfo,
}

impl<'a> BoneInfo<'a> {
    fn of((index, bone): (usize, &'a Bone)) -> Self {
        Self {
            index,
            name: &bone.name,
            parent: bone.parent,
            controller_id: bone.controller_id,
            node: bone.node,
            relative: PoseInfo::of(&bone.relative),
            world: PoseInfo::of(&bone.world),
        }
    }
}

#[derive(Debug, Serialize)]
struct PoseInfo {
    rotation: [f32; 4],
    translation: [f32; 3],
}

impl PoseInfo {
    const fn of(pose: &BoneTransform) -> Self {
        Self {
            rotation: pose.rotation,
            translation: pose.translation,
        }
    }
}

/// One chunk of a model's files.
#[derive(Debug, Serialize)]
pub struct ChunkInfo {
    file: Cow<'static, str>,
    index: usize,
    #[serde(rename = "type")]
    type_name: Option<&'static str>,
    type_id: u32,
    usage: &'static str,
}

impl ChunkInfo {
    /// Describes `chunk`.
    #[must_use]
    pub fn of(chunk: &ModelChunk) -> Self {
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
#[must_use]
pub fn file_name(file: ModelFile) -> Cow<'static, str> {
    match file {
        ModelFile::Model => Cow::Borrowed("model"),
        ModelFile::Geometry => Cow::Borrowed("geometry"),
        ModelFile::LodGeometry(level) => Cow::Owned(format!("lod{level}-geometry")),
        _ => Cow::Borrowed("other"),
    }
}

/// Returns `read`, `skipped` or `unknown`.
#[must_use]
pub const fn usage_name(usage: ChunkUsage) -> &'static str {
    match usage {
        ChunkUsage::Read => "read",
        ChunkUsage::Skipped => "skipped",
        ChunkUsage::Unknown => "unknown",
    }
}

/// Returns the name of the node `index`, or `-` when it has none.
#[must_use]
pub fn node_name(model: &Model, index: usize) -> &str {
    model
        .nodes
        .get(index)
        .map(|node| node.name.as_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("-")
}
