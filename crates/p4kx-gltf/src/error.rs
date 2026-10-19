use std::io;

use thiserror::Error;

/// A failure to write a GLB file.
#[derive(Debug, Error)]
pub enum GltfError {
    /// The model has no level of detail with the requested number.
    #[error("the model has no LOD{lod}; it has {available}")]
    NoSuchLod {
        /// The requested level.
        lod: u32,
        /// The levels the model has, e.g. `LOD0`.
        available: String,
    },
    /// The model does not fit in a GLB file, whose length is a 32-bit number.
    #[error("the model needs {len} bytes, more than a GLB file can hold")]
    TooLarge {
        /// The length the file would need.
        len: u64,
    },
    /// The glTF JSON could not be serialised.
    #[error("cannot serialise the glTF JSON")]
    Json(#[from] serde_json::Error),
    /// The output could not be written.
    #[error("cannot write the GLB file")]
    Write(#[from] io::Error),
}
