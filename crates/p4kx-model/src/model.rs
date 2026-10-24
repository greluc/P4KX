// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_chunks::{ChunkFile, ChunkType, Container, ivo};

use crate::build::build;
use crate::crch::build as build_crch;
use crate::{Bone, CompanionResolver, Influences, MaterialStatus, ModelError, Transform};

/// A model read from a chunk file and its companions, in `CryEngine` coordinates: Z up, metres.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Model {
    /// The model's file name.
    pub name: String,
    /// The file name of the geometry companion of level 0, or `None` when its meshes are in the
    /// model file itself or level 0 was not read.
    pub geometry: Option<String>,
    /// The nodes; a model without a `NodeMeshCombo` chunk has one root node named after the file.
    pub nodes: Vec<Node>,
    /// The levels of detail, LOD0 first.
    pub lods: Vec<Lod>,
    /// The material library the material IDs refer to, a path below `Data` without `.mtl`.
    pub material_library: Option<String>,
    /// What reading the material library came to; a missing or unreadable library leaves the
    /// model readable, with the engine's default material.
    pub materials: MaterialStatus,
    /// The bones of the skeleton from the model file's `CompiledBones2` chunk (a skin or a
    /// skeleton file) or its `CompiledBones` chunk (a static model with animated parts); empty
    /// for a model with neither.
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

/// One level of detail: the meshes of the nodes at that level.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Lod {
    /// The level, 0 for the most detailed.
    pub level: u32,
    /// The meshes, at most one per node, in node order.
    pub meshes: Vec<Mesh>,
    /// The skeleton the bone indices of this level's meshes refer to, when the level's own
    /// `<stem>_lod<n>.<extension>` file carries one that differs from [`Model::bones`]; see
    /// [`Model::bones_of`].
    pub bones: Option<Vec<Bone>>,
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
    /// The unit vertex tangents with the handedness of their frame in `w`, +1 or −1, or `None`
    /// when the mesh stores no tangent frame. The bitangent is `cross(normal, tangent) * w`.
    pub tangents: Option<Vec<[f32; 4]>>,
    /// The vertex colours as red, green, blue and alpha, or `None` when every vertex is opaque
    /// white, the colour of a mesh that stores none.
    pub colours: Option<Vec<[u8; 4]>>,
    /// The bones each vertex is weighted to, one entry per vertex, or `None` for a mesh that is
    /// not skinned. Every bone with a weight is in [`Model::bones`].
    pub influences: Option<Vec<Influences>>,
    /// The texture coordinates, one per vertex, or empty when the mesh stores none.
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
#[non_exhaustive]
pub enum ModelFile {
    /// The model file itself, e.g. the `.cgf`.
    Model,
    /// The geometry companion, e.g. the `.cgfm`.
    Geometry,
    /// The geometry companion of a level of detail from 1 on, e.g. `_lod1.cgfm`.
    LodGeometry(u32),
}

/// Which levels of detail [`Model::read_with`] reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LodSelection {
    /// Every level the model has.
    #[default]
    All,
    /// Only this level; reading fails when the model lacks it.
    Only(u32),
    /// Every level up to and including this one that the model has; reading fails when it has
    /// none of them.
    AtMost(u32),
}

impl LodSelection {
    /// Returns whether the selection includes `level`.
    #[must_use]
    pub const fn wants(self, level: u32) -> bool {
        match self {
            Self::All => true,
            Self::Only(only) => only == level,
            Self::AtMost(most) => level <= most,
        }
    }
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
    /// `companions` unless the file holds its mesh itself; a file without `MeshInfo` chunk whose
    /// companion does not exist is read as helper nodes without meshes. A `CrCh` file holds its
    /// meshes itself and needs `companions` only for its material library.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::MissingGeometry`], [`ModelError::Companion`] or
    /// [`ModelError::CompanionChunks`] when the companion is missing or unreadable,
    /// [`ModelError::NoMesh`] when the companion holds no mesh, and [`ModelError::Corrupt`] when a chunk
    /// the model reads is corrupt.
    pub fn read<B: AsRef<[u8]>>(
        name: &str,
        file: &ChunkFile<B>,
        companions: &mut dyn CompanionResolver,
    ) -> Result<Self, ModelError> {
        Self::read_with(name, file, companions, LodSelection::All)
    }

    /// Reads the levels of detail `lods` of the model `file` named `name`. Level 0 is in the
    /// geometry companion `<name>m`, level `n` from 1 on in `<stem>_lod<n>.<extension>m`; a level
    /// whose companion does not exist is left out. The nodes always come from `file`.
    ///
    /// # Errors
    ///
    /// As [`Self::read`], and [`ModelError::NoSuchLod`] when `lods` is [`LodSelection::Only`]
    /// and the model lacks that level, or [`LodSelection::AtMost`] and it has none up to it.
    pub fn read_with<B: AsRef<[u8]>>(
        name: &str,
        file: &ChunkFile<B>,
        companions: &mut dyn CompanionResolver,
        lods: LodSelection,
    ) -> Result<Self, ModelError> {
        Self::read_selecting(name, file, companions, lods, true)
    }

    /// Reads the levels of detail `lods` of the model `file` like [`Self::read_with`], but leaves
    /// the material library the model names unread: [`Self::materials`] is
    /// [`MaterialStatus::Skipped`] and [`Self::material_library`] still holds the name, for a
    /// caller that chooses and caches the library itself, see [`MaterialStatus::load`].
    ///
    /// # Errors
    ///
    /// As [`Self::read_with`].
    pub fn read_without_materials<B: AsRef<[u8]>>(
        name: &str,
        file: &ChunkFile<B>,
        companions: &mut dyn CompanionResolver,
        lods: LodSelection,
    ) -> Result<Self, ModelError> {
        Self::read_selecting(name, file, companions, lods, false)
    }

    fn read_selecting<B: AsRef<[u8]>>(
        name: &str,
        file: &ChunkFile<B>,
        companions: &mut dyn CompanionResolver,
        lods: LodSelection,
        load_materials: bool,
    ) -> Result<Self, ModelError> {
        match file.container() {
            Container::Ivo => build(name, file, companions, lods, load_materials),
            Container::CrCh => build_crch(name, file, companions, lods, load_materials),
        }
    }

    /// Returns the bones the meshes of `lod` are weighted to: those of the level's own file when
    /// it differs, otherwise those of the model file.
    #[must_use]
    pub fn bones_of<'a>(&'a self, lod: &'a Lod) -> &'a [Bone] {
        lod.bones.as_deref().unwrap_or(&self.bones)
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
