//! Reads the DDS textures of Star Citizen Archives, which store a texture as a header Entry
//! holding the DDS header and the smallest mips plus one sibling Entry `.dds.N` per larger mip,
//! `.dds.1` holding the smallest of them. Parses legacy and DX10 headers and merges the parts
//! into one standard DDS file through a [`SiblingReader`] the caller supplies.

mod dxgi;
mod error;
mod format;
mod header;
mod merge;

pub use dxgi::DxgiFormat;
pub use error::TextureError;
pub use format::{BlockLayout, TextureFormat};
pub use header::{DdsHeader, MAX_HEADER_LEN};
pub use merge::{MergeReport, SiblingReader, merge};
