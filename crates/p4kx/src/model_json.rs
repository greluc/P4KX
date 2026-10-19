use p4kx_extraction::ModelSource;
use p4kx_extraction::model::{ChunkUsage, Mesh, Model, ModelChunk, ModelFile, Node, Submesh};
use serde::Serialize;

/// The JSON shape of `p4kx model info`.
#[derive(Serialize)]
pub(crate) struct InfoJson<'a> {
    model: &'a str,
    geometry: Option<&'a str>,
    material_library: Option<&'a str>,
    material_ids: Vec<u16>,
    nodes: Vec<NodeJson<'a>>,
    lods: Vec<LodJson>,
    bones: Vec<BoneJson<'a>>,
    chunks: Vec<ChunkJson>,
}

impl<'a> InfoJson<'a> {
    pub(crate) fn of(source: &'a ModelSource) -> Self {
        let model = source.model();
        Self {
            model: source.name(),
            geometry: model.geometry.as_deref(),
            material_library: model.material_library.as_deref(),
            material_ids: model.material_ids(),
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
            bones: model
                .bones
                .iter()
                .map(|bone| BoneJson {
                    name: &bone.name,
                    parent: bone.parent,
                })
                .collect(),
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
struct MeshJson {
    node: usize,
    vertices: usize,
    triangles: usize,
    normals: bool,
    submeshes: Vec<SubmeshJson>,
}

impl MeshJson {
    fn of(mesh: &Mesh) -> Self {
        Self {
            node: mesh.node,
            vertices: mesh.positions.len(),
            triangles: mesh.indices.len() / 3,
            normals: mesh.normals.is_some(),
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
    name: &'a str,
    parent: Option<usize>,
}

/// The JSON shape of one chunk of a model's files.
#[derive(Serialize)]
pub(crate) struct ChunkJson {
    file: &'static str,
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

/// Returns `model` or `geometry`.
pub(crate) const fn file_name(file: ModelFile) -> &'static str {
    match file {
        ModelFile::Model => "model",
        ModelFile::Geometry => "geometry",
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
