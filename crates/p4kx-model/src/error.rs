use p4kx_chunks::ChunkError;
use p4kx_foundation::ParseError;
use thiserror::Error;

use crate::CompanionError;

/// A failure to read a model.
#[derive(Debug, Error)]
pub enum ModelError {
    /// The model holds no mesh and its geometry companion does not exist.
    #[error("the geometry companion {file} does not exist")]
    MissingGeometry {
        /// The companion's file name.
        file: String,
    },
    /// A companion exists but cannot be read.
    #[error("cannot read the companion {file}")]
    Companion {
        /// The companion's file name.
        file: String,
        /// The resolver's failure.
        #[source]
        source: CompanionError,
    },
    /// A companion is no valid chunk file.
    #[error("{file}")]
    CompanionChunks {
        /// The companion's file name.
        file: String,
        /// What is wrong.
        #[source]
        source: ChunkError,
    },
    /// Neither the model nor its geometry companion holds a mesh chunk.
    #[error("{file} holds no Skin or Skin2 mesh chunk")]
    NoMesh {
        /// The file the mesh was expected in.
        file: String,
    },
    /// The model has no level of detail with the requested number.
    #[error("{file} has no LOD{lod}")]
    NoSuchLod {
        /// The model file.
        file: String,
        /// The requested level.
        lod: u32,
    },
    /// A chunk the model reads is corrupt.
    #[error("{file}: chunk {index} ({chunk}) is corrupt")]
    Corrupt {
        /// The file holding the chunk.
        file: String,
        /// The chunk's position in the file's chunk table.
        index: usize,
        /// The chunk type's name.
        chunk: &'static str,
        /// What is wrong.
        #[source]
        source: CorruptChunk,
    },
}

/// What is wrong with a chunk a model reads.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CorruptChunk {
    /// The chunk ends before a value it declares.
    #[error("it is truncated")]
    Truncated(#[from] ParseError),
    /// A data stream has an element size its kind never has.
    #[error("the {stream} stream has {size}-byte elements")]
    ElementSize {
        /// The stream kind.
        stream: &'static str,
        /// The element size found.
        size: u32,
    },
    /// A data stream every mesh needs is missing.
    #[error("it has no {0} stream")]
    MissingStream(&'static str),
    /// A data stream the model reads occurs twice.
    #[error("the stream {0:#010X} occurs twice")]
    RepeatedStream(u32),
    /// A submesh's index range reaches past the indices.
    #[error("submesh {submesh} covers indices {first}..{first}+{count} of {total}")]
    SubmeshRange {
        /// The submesh's position.
        submesh: usize,
        /// Its first index.
        first: u32,
        /// Its number of indices.
        count: u32,
        /// The number of indices of the mesh.
        total: u32,
    },
    /// The submeshes draw more indices than the mesh has.
    #[error("the submeshes draw {drawn} indices of {indices}")]
    IndexTotal {
        /// The indices drawn by the submeshes so far.
        drawn: u64,
        /// The number of indices of the mesh.
        indices: u32,
    },
    /// A submesh's index count is no whole number of triangles.
    #[error("submesh {submesh} has {count} indices, which are no whole triangles")]
    NotTriangles {
        /// The submesh's position.
        submesh: usize,
        /// Its number of indices.
        count: u32,
    },
    /// A submesh refers to a vertex the mesh does not have.
    #[error("submesh {submesh} refers to vertex {index} of {vertices}")]
    IndexRange {
        /// The submesh's position.
        submesh: usize,
        /// The vertex index, its page base added.
        index: u64,
        /// The number of vertices.
        vertices: u32,
    },
    /// A submesh belongs to a node the model does not have.
    #[error("submesh {submesh} belongs to node {node} of {nodes}")]
    NodeRange {
        /// The submesh's position.
        submesh: usize,
        /// The node index.
        node: u16,
        /// The number of nodes.
        nodes: usize,
    },
    /// A node's parent does not exist.
    #[error("node {node} has the parent {parent} of {nodes}")]
    ParentRange {
        /// The node's position.
        node: usize,
        /// The parent index.
        parent: u16,
        /// The number of nodes.
        nodes: usize,
    },
    /// A node is its own ancestor.
    #[error("node {0} is its own ancestor")]
    ParentCycle(usize),
    /// A value is NaN or infinite.
    #[error("a {0} is not finite")]
    NonFinite(&'static str),
    /// A `CrCh` chunk is big-endian, which no model file of the Archives is.
    #[error("its data is big-endian, which is not supported")]
    BigEndian,
    /// Two `CrCh` nodes carry the same chunk ID.
    #[error("the chunk ID {0} belongs to two nodes")]
    RepeatedChunkId(u32),
    /// A `CrCh` node names a parent chunk ID no node has.
    #[error("node {node} has the parent chunk ID {parent}, which no node has")]
    ParentId {
        /// The node's position.
        node: usize,
        /// The parent chunk ID.
        parent: u32,
    },
    /// A chunk refers to a chunk ID the file does not hold, or one of the wrong type.
    #[error("it refers to the chunk ID {0}, which is missing or of another type")]
    ReferencedChunk(u32),
    /// A `CrCh` mesh stores vertices, indices or subsets in data streams, which no model file of
    /// the Archives does.
    #[error(
        "it stores {vertices} vertices and {indices} indices in data streams, which is not supported"
    )]
    StreamedMesh {
        /// The number of vertices the header declares.
        vertices: u32,
        /// The number of indices the header declares.
        indices: u32,
    },
    /// A physics mesh chunk holds another primitive than a polyhedron, the one the Archives hold.
    #[error("its physics data has the primitive type {0}, which is not supported")]
    PhysicsPrimitive(u32),
    /// A physics mesh declares more vertices or triangles than its data holds.
    #[error(
        "the physics mesh declares {vertices} vertices and {triangles} triangles for {len} bytes"
    )]
    PhysicsCounts {
        /// The declared number of vertices.
        vertices: u32,
        /// The declared number of triangles.
        triangles: u32,
        /// The length of the physics mesh data.
        len: usize,
    },
    /// A physics mesh triangle refers to a vertex the mesh does not have.
    #[error("the physics mesh refers to vertex {index} of {vertices}")]
    PhysicsIndex {
        /// The vertex index.
        index: u32,
        /// The number of vertices.
        vertices: u32,
    },
}
