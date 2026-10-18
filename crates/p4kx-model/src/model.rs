use p4kx_chunks::{ChunkFile, ChunkType, Container, ivo};

use crate::build::build;
use crate::{CompanionResolver, ModelError, Transform};

/// A model read from a chunk file and its companions, in `CryEngine` coordinates: Z up, metres.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Model {
    /// The model's file name.
    pub name: String,
    /// The file name of the geometry companion the meshes come from, or `None` when they are in
    /// the model file itself.
    pub geometry: Option<String>,
    /// The nodes; a model without a `NodeMeshCombo` chunk has one root node named after the file.
    pub nodes: Vec<Node>,
    /// The levels of detail, LOD0 first.
    pub lods: Vec<Lod>,
    /// The material library the material IDs refer to, a path below `Data` without `.mtl`.
    pub material_library: Option<String>,
    /// The bones of the skeleton; empty for a static model.
    pub bones: Vec<Bone>,
    /// Every chunk of the model file and its geometry companion, with how the model used it.
    pub chunks: Vec<ModelChunk>,
}

/// One node of a model's hierarchy.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Node {
    /// The node's name; may be empty.
    pub name: String,
    /// The index of the parent node, or `None` for a root.
    pub parent: Option<usize>,
    /// The transform from the node's space to its parent's.
    pub transform: Transform,
}

/// One bone of a skeleton.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Bone {
    /// The bone's name.
    pub name: String,
    /// The index of the parent bone, or `None` for the root.
    pub parent: Option<usize>,
}

/// One level of detail: the meshes of the nodes at that level.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Lod {
    /// The level, 0 for the most detailed.
    pub level: u32,
    /// The meshes, at most one per node, in node order.
    pub meshes: Vec<Mesh>,
}

impl Lod {
    /// Returns the number of vertices of all meshes.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.meshes.iter().map(|mesh| mesh.positions.len()).sum()
    }

    /// Returns the number of triangles of all meshes.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.meshes.iter().map(|mesh| mesh.indices.len() / 3).sum()
    }
}

/// The triangles of one node at one level of detail, in the node's space.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Mesh {
    /// The index of the node the mesh belongs to.
    pub node: usize,
    /// The vertex positions in metres.
    pub positions: Vec<[f32; 3]>,
    /// The unit vertex normals, or `None` when the mesh stores none.
    pub normals: Option<Vec<[f32; 3]>>,
    /// The texture coordinates, one per vertex.
    pub uvs: Vec<[f32; 2]>,
    /// The triangle list: three vertex indices per triangle, every one below the vertex count.
    pub indices: Vec<u32>,
    /// The ranges of the triangle list drawn with one material each.
    pub submeshes: Vec<Submesh>,
}

/// A range of a mesh's triangle list drawn with one material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Submesh {
    /// The material ID: the position of the sub-material in the material library.
    pub material_id: u16,
    /// The first index of the range in the mesh's triangle list.
    pub first_index: usize,
    /// The number of indices, a multiple of three.
    pub index_count: usize,
}

/// Which file of a model a chunk is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelFile {
    /// The model file itself, e.g. the `.cgf`.
    Model,
    /// The geometry companion, e.g. the `.cgfm`.
    Geometry,
}

/// How a model used one of its chunks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChunkUsage {
    /// The model was read from the chunk.
    Read,
    /// The chunk type is known, but the model does not read it.
    Skipped,
    /// The chunk type is unknown and was skipped.
    Unknown,
}

/// One chunk of a model's files and how the model used it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ModelChunk {
    /// The file holding the chunk.
    pub file: ModelFile,
    /// The chunk's position in the file's chunk table.
    pub index: usize,
    /// The chunk's type.
    pub chunk_type: ChunkType,
    /// How the model used it.
    pub usage: ChunkUsage,
}

impl Model {
    /// Reads the model `file`, named `name`, fetching its geometry companion `<name>m` from
    /// `companions` unless the file holds its mesh itself.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::UnsupportedContainer`] for a `CrCh` file,
    /// [`ModelError::MissingGeometry`], [`ModelError::Companion`] or
    /// [`ModelError::CompanionChunks`] when the companion is missing or unreadable,
    /// [`ModelError::NoMesh`] when no file holds a mesh, and [`ModelError::Corrupt`] when a chunk
    /// the model reads is corrupt.
    pub fn read<B: AsRef<[u8]>>(
        name: &str,
        file: &ChunkFile<B>,
        companions: &mut dyn CompanionResolver,
    ) -> Result<Self, ModelError> {
        if file.container() != Container::Ivo {
            return Err(ModelError::UnsupportedContainer {
                file: name.to_owned(),
                container: file.container(),
            });
        }
        build(name, file, companions)
    }

    /// Returns the chunks of unknown type, which the model skipped.
    pub fn unknown_chunks(&self) -> impl Iterator<Item = &ModelChunk> {
        self.chunks
            .iter()
            .filter(|chunk| chunk.usage == ChunkUsage::Unknown)
    }

    /// Returns the material IDs the submeshes of every level of detail use, ascending.
    #[must_use]
    pub fn material_ids(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = self
            .lods
            .iter()
            .flat_map(|lod| &lod.meshes)
            .flat_map(|mesh| &mesh.submeshes)
            .map(|submesh| submesh.material_id)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }
}

/// The chunk types holding a mesh, preferred first.
pub(crate) const MESH_TYPES: [ChunkType; 2] = [ivo::SKIN_2, ivo::SKIN];
