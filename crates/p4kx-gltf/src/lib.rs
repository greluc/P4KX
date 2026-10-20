//! Writes a [`p4kx_model::Model`] as one glTF 2.0 binary file (GLB).
//!
//! [`write_glb`] converts the model from `CryEngine` coordinates (Z up) to glTF's (Y up), keeps
//! metres, turns every node into a glTF node with its transform and every mesh into a glTF mesh
//! with one primitive per submesh. The model's material library becomes glTF materials: with
//! [`MaterialMode::Colours`] plain colours, opacity, roughness, metalness and emissive colour
//! without textures; submeshes whose material draws nothing are left out, and a material ID the
//! library lacks, or a model without a readable library, gets the engine's default material. A
//! skinned model is written with its skeleton: the bones as nodes, one glTF skin over all of them
//! with inverse bind matrices, and `JOINTS_n` and `WEIGHTS_n` on the skinned meshes; a model with
//! bones and no meshes is written as its bone nodes. [`write_glb_animated`] adds glTF animations
//! that move those bone nodes, or the nodes a static model's helper bones drive, with rotation and
//! translation channels. The binary data is streamed to the writer after the JSON.

mod animations;
mod axes;
mod block;
mod document;
mod error;
mod glb;
mod layout;
mod materials;
mod nodes;
mod skin;

pub use animations::AnimationSummary;
pub use error::GltfError;
pub use glb::{GlbOptions, GlbSummary, MaterialMode, write_glb, write_glb_animated};
