use p4kx_chunks::{ChunkError, Container};
use p4kx_foundation::ParseError;
use thiserror::Error;

use crate::CompanionError;

/// A failure to read a model.
#[derive(Debug, Error)]
pub enum ModelError {
    /// The model is in a container whose chunk types are not interpreted yet.
    #[error("{file}: models in the {container} container are not supported")]
    UnsupportedContainer {
        /// The model file.
        file: String,
        /// Its container form.
        container: Container,
    },
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
}
