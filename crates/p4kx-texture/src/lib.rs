//! Reads the DDS textures of Star Citizen Archives, which store a texture as a header Entry
//! holding the DDS header and the smallest mips plus one sibling Entry `.dds.N` per larger mip,
//! `.dds.1` holding the smallest of them. Parses legacy and DX10 headers, merges the parts
//! into one standard DDS file through a [`SiblingReader`] the caller supplies, reads and decodes
//! single mips to RGBA and writes them as PNG.

mod bc;
mod dxgi;
mod error;
mod format;
mod header;
mod image;
mod merge;
mod mip;
mod pixels;
mod png_export;

pub use dxgi::DxgiFormat;
pub use error::TextureError;
pub use format::{BlockLayout, TextureFormat};
pub use header::{DdsHeader, MAX_HEADER_LEN};
pub use image::{RgbaImage, decode};
pub use merge::{MergeReport, SiblingReader, merge};
pub use mip::{Mip, read_mip};
pub use png_export::write_png;
