// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_texture::{TextureError, write_png};

use crate::{ConversionError, Texture};

/// A preview of one mip of a texture, encoded as an 8-bit RGBA PNG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewPng {
    /// The mip level shown, 0 being the largest.
    pub level: u32,
    /// The width of the image in pixels.
    pub width: u32,
    /// The height of the image in pixels.
    pub height: u32,
    /// The PNG file.
    pub png: Vec<u8>,
}

impl Texture<'_> {
    /// Decodes mip `level` with its alpha tail merged in and encodes it as one PNG: the image
    /// [`Texture::decode_preview`] gives, or with `face` the one face of a cubemap, counted in DDS
    /// order +X, −X, +Y, −Y, +Z, −Z.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Texture::decode_preview`] and [`Texture::decode_surfaces`], and a
    /// [`ConversionError`] wrapping [`TextureError::UnsupportedImage`] when the texture has no
    /// such face.
    pub fn preview_png(
        &self,
        level: u32,
        face: Option<usize>,
    ) -> Result<PreviewPng, ConversionError> {
        let image = match face {
            None => self.decode_preview(level)?,
            Some(face) => {
                let mut images = self.decode_surfaces(level)?;
                if face >= images.len() || self.header().faces() < 2 {
                    return Err(ConversionError::of_texture(
                        self.name(),
                        TextureError::UnsupportedImage {
                            images: self.header().images(level),
                        },
                    ));
                }
                images.swap_remove(face)
            }
        };
        let mut png = Vec::new();
        write_png(&image, &mut png)
            .map_err(|error| ConversionError::of_texture(self.name(), error))?;
        Ok(PreviewPng {
            level,
            width: image.width(),
            height: image.height(),
            png,
        })
    }
}
