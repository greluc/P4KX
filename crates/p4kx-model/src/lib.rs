//! Interprets `CryEngine` chunk files as models.
//!
//! [`Model::read`] builds a [`Model`] from a parsed [`p4kx_chunks::ChunkFile`] and the companion
//! files a [`CompanionResolver`] supplies: its [`Node`]s with their transforms and hierarchy, the
//! [`Mesh`]es of each [`Lod`] with positions, normals, tangents, texture coordinates, vertex
//! colours and indices, and the
//! [`Submesh`]es that draw them with one material ID each, and the [`MaterialLibrary`] of the
//! `.mtl` file the IDs index. Coordinates stay in the `CryEngine`
//! convention, Z up, in metres. Chunks the model does not read are listed with their
//! [`ChunkUsage`].
//!
//! Both containers are read: an `#ivo` model keeps its meshes in a geometry companion, a `CrCh`
//! model in its own file, as the physics mesh a `Mesh` chunk refers to (docs/crch-models.md).

mod build;
mod companion;
mod crch;
mod crch_mesh;
mod crch_node;
mod error;
mod material;
mod mesh;
mod model;
mod mtl;
mod mtl_error;
mod mtl_load;
mod mtl_parse;
mod mtl_xml;
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
pub use mtl::{
    MAX_SUB_MATERIALS, MAX_TEXTURE_SLOTS, Material, MaterialFlags, MaterialLibrary, MaterialStatus,
    TextureSlot,
};
pub use mtl_error::MaterialError;
pub use transform::Transform;
