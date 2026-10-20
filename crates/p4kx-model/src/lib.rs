//! Interprets `CryEngine` chunk files as models.
//!
//! [`Model::read`] builds a [`Model`] from a parsed [`p4kx_chunks::ChunkFile`] and the companion
//! files a [`CompanionResolver`] supplies: its [`Node`]s with their transforms and hierarchy, the
//! [`Mesh`]es of each [`Lod`] with positions, normals, tangents, texture coordinates, vertex
//! colours, bone weights ([`Influences`]) and indices, the [`Bone`]s of the skeleton with their
//! bind poses, and the
//! [`Submesh`]es that draw them with one material ID each, and the [`MaterialLibrary`] of the
//! `.mtl` file the IDs index. Coordinates stay in the `CryEngine`
//! convention, Z up, in metres. Chunks the model does not read are listed with their
//! [`ChunkUsage`].
//!
//! [`Animation::read_caf`] and [`Dba::read`] read the animations of `.caf` files and `.dba`
//! animation databases into [`Track`]s of rotation and translation [`Keys`] per controller;
//! [`Animation::bind`] matches them to the [`Bone`]s of a skeleton by controller ID.

mod animation;
mod animation_error;
mod animation_keys;
mod animation_read;
mod bone;
mod bone_map;
mod build;
mod companion;
mod error;
mod hierarchy;
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
mod skeleton;
mod skin;
mod streams;
mod transform;
mod vector;

pub use animation::{Animation, BoundTracks, Keys, Track};
pub use animation_error::{AnimationError, CorruptAnimation};
pub use animation_read::{Dba, MAX_ANIMATION_KEYS, MAX_DATABASE_KEYS, SkippedAnimation};
pub use bone::{Bone, BoneTransform};
pub use bone_map::{Influences, MAX_INFLUENCES};
pub use companion::{CompanionError, CompanionPath, CompanionResolver};
pub use error::{CorruptChunk, ModelError};
pub use model::{ChunkUsage, Lod, LodSelection, Mesh, Model, ModelChunk, ModelFile, Node, Submesh};
pub use mtl::{
    MAX_SUB_MATERIALS, MAX_TEXTURE_SLOTS, Material, MaterialFlags, MaterialLibrary, MaterialStatus,
    TextureSlot,
};
pub use mtl_error::MaterialError;
pub use transform::Transform;
