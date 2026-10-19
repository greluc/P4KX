//! Writes a [`p4kx_model::Model`] as one glTF 2.0 binary file (GLB).
//!
//! [`write_glb`] converts the model from `CryEngine` coordinates (Z up) to glTF's (Y up), keeps
//! metres, turns every node into a glTF node with its transform and every mesh into a glTF mesh
//! with one primitive per submesh, and gives every material ID a material of its own. The binary
//! data is streamed to the writer after the JSON.

mod axes;
mod document;
mod error;
mod glb;
mod layout;

pub use error::GltfError;
pub use glb::{GlbOptions, GlbSummary, write_glb};
