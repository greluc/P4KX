//! Interprets `CryEngine` chunk files as models.
//!
//! [`Model::read`] builds a [`Model`] from a parsed [`p4kx_chunks::ChunkFile`] and the companion
//! files a [`CompanionResolver`] supplies: its [`Node`]s with their transforms and hierarchy, the
//! [`Mesh`]es of each [`Lod`] with positions, normals, tangents, texture coordinates, vertex
//! colours and indices, and the
//! [`Submesh`]es that draw them with one material ID each. Coordinates stay in the `CryEngine`
//! convention, Z up, in metres. Chunks the model does not read are listed with their
//! [`ChunkUsage`].

mod build;
mod companion;
mod error;
mod material;
mod mesh;
mod model;
mod nmc;
mod reader;
mod skin;
mod streams;
mod transform;
mod vector;

pub use companion::{CompanionError, CompanionPath, CompanionResolver};
pub use error::{CorruptChunk, ModelError};
pub use model::{
    Bone, ChunkUsage, Lod, LodSelection, Mesh, Model, ModelChunk, ModelFile, Node, Submesh,
};
pub use transform::Transform;
