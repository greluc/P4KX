//! Reads the DDS textures of Star Citizen Archives, which store a texture as a header Entry
//! holding the DDS header and the smallest mips plus one sibling Entry `.dds.N` per larger mip,
//! `.dds.1` holding the smallest of them. Parses legacy and DX10 headers, merges the parts
//! into one standard DDS file through a [`SiblingReader`] the caller supplies, reads and decodes
//! single mips to RGBA, BC5 normal maps with their Z reconstructed, and writes them as PNG;
//! decodes BC6H to floating-point RGB as well and writes that as OpenEXR.

mod bc;
mod bc6h;
mod bc6h_modes;
mod bc7;
mod bc7_tables;
mod dxgi;
mod error;
mod exr_export;
mod format;
mod hdr;
mod header;
mod image;
mod merge;
mod mip;
mod pixels;
mod png_export;
mod rgtc;
mod tone_map;

pub use dxgi::DxgiFormat;
pub use error::TextureError;
pub use exr_export::write_exr;
pub use format::{BlockLayout, TextureFormat};
pub use hdr::{HdrImage, decode_hdr};
pub use header::{DdsHeader, MAX_HEADER_LEN};
pub use image::{RgbaImage, decode};
pub use merge::{MergeReport, SiblingReader, merge};
pub use mip::{Mip, read_mip};
pub use png_export::write_png;
