use std::path::Path;

use p4kx_texture::{HdrImage, TextureError, write_exr};

use crate::output_file::write_replacing;
use crate::{ConversionError, Texture};

/// What an OpenEXR export wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExrReport {
    /// The exported mip level, 0 being the largest.
    pub level: u32,
    /// Width of the image in pixels.
    pub width: u32,
    /// Height of the image in pixels.
    pub height: u32,
}

impl Texture<'_> {
    /// Reads and decodes mip `level`, 0 being the largest, of a high-range texture to linear
    /// floating-point RGB without clipping its range.
    ///
    /// # Errors
    ///
    /// Returns a [`ConversionError`] wrapping [`TextureError::NotHighRange`] before reading
    /// anything when the format is not high-range, and the errors of [`Texture::read_mip`].
    pub fn decode_mip_hdr(&self, level: u32) -> Result<HdrImage, ConversionError> {
        let format = self.header().format();
        if !format.is_high_range() {
            return Err(ConversionError::of_texture(
                self.name(),
                TextureError::NotHighRange(format),
            ));
        }
        self.read_mip(level)?
            .decode_hdr()
            .map_err(|error| ConversionError::of_texture(self.name(), error))
    }

    /// Decodes mip `level`, 0 being the largest, of a high-range texture and writes it to the
    /// file `out` as OpenEXR with 32-bit float RGB, replacing it; a failed export leaves nothing
    /// behind.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::OutputIsInput`] when `out` is the header or one of its parts,
    /// [`ConversionError::Write`] when the file cannot be written, and the errors of
    /// [`Texture::decode_mip_hdr`].
    pub fn exr_to_file(&self, out: &Path, level: u32) -> Result<ExrReport, ConversionError> {
        self.refuse_input(out)?;
        let image = self.decode_mip_hdr(level)?;
        write_replacing(out, |writer| {
            write_exr(&image, writer)
                .map_err(|error| ConversionError::of_texture(self.name(), error))
        })?;
        Ok(ExrReport {
            level,
            width: image.width(),
            height: image.height(),
        })
    }
}
