//! Writes a [`p4kx_model::Model`] as one glTF 2.0 binary file (GLB).
//!
//! [`write_glb`] converts the model from `CryEngine` coordinates (Z up) to glTF's (Y up), keeps
//! metres, turns every node into a glTF node with its transform and every mesh into a glTF mesh
//! with one primitive per submesh. The model's material library becomes glTF materials: with
//! [`MaterialMode::Colours`] plain colours, opacity, roughness, metalness and emissive colour
//! without textures; submeshes whose material draws nothing are left out, and a material ID the
//! library lacks, or a model without a readable library, gets the engine's default material. The
//! binary data is streamed to the writer after the JSON.
//!
//! [`write_scene_glb`] writes a [`Scene`]: a tree of named, placed nodes that hold models, each
//! drawn with the material library the scene gives it. Equal materials are written once.

mod axes;
mod document;
mod error;
mod glb;
mod layout;
mod materials;
mod scene;

pub use error::GltfError;
pub use glb::{GlbOptions, GlbSummary, MaterialMode, write_glb, write_scene_glb};
pub use scene::{Scene, SceneNode, ScenePart};
