// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};

use p4kx_texture::{HdrImage, TextureError, cube_cross_hdr, write_exr};
use rayon::prelude::*;

use crate::output_file::write_replacing;
use crate::texture_png::{CUBE_FACE_NAMES, face_path, is_cube};
use crate::{ConversionError, CubemapLayout, Texture};

/// What an OpenEXR export wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExrReport {
    /// The exported mip level, 0 being the largest.
    pub level: u32,
    /// Width of each image in pixels.
    pub width: u32,
    /// Height of each image in pixels.
    pub height: u32,
    /// The written files: one, or six faces of a cubemap.
    pub files: Vec<PathBuf>,
}

impl Texture<'_> {
    /// Reads and decodes mip `level`, 0 being the largest, of a high-range texture to linear
    /// floating-point RGB without clipping its range.
    ///
    /// # Errors
    ///
    /// Returns a [`ConversionError`] wrapping [`TextureError::UnsupportedImage`] when the mip
    /// holds several images, and the errors of [`Texture::decode_surfaces_hdr`].
    pub fn decode_mip_hdr(&self, level: u32) -> Result<HdrImage, ConversionError> {
        self.check_high_range()?;
        self.read_mip(level)?
            .decode_hdr()
            .map_err(|error| ConversionError::of_texture(self.name(), error))
    }

    /// Reads and decodes every image of mip `level` of a high-range texture, a cubemap's six
    /// faces in DDS order, to linear floating-point RGB. An alpha tail is not read, as the
    /// images have no alpha channel.
    ///
    /// # Errors
    ///
    /// Returns a [`ConversionError`] wrapping [`TextureError::NotHighRange`] before reading
    /// anything when the format is not high-range, and the errors of [`Texture::read_mip`].
    pub fn decode_surfaces_hdr(&self, level: u32) -> Result<Vec<HdrImage>, ConversionError> {
        self.check_high_range()?;
        self.read_mip(level)?
            .decode_surfaces_hdr()
            .map_err(|error| ConversionError::of_texture(self.name(), error))
    }

    /// Decodes mip `level`, 0 being the largest, of a high-range texture and writes it to the
    /// file `out` as OpenEXR with 32-bit float RGB, replacing it; a cubemap is written as six
    /// face files.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Texture::exr_to_file_as`].
    pub fn exr_to_file(&self, out: &Path, level: u32) -> Result<ExrReport, ConversionError> {
        self.exr_to_file_as(out, level, CubemapLayout::Faces)
    }

    /// Decodes mip `level` of a high-range texture and writes it as OpenEXR to the file `out`,
    /// or a cubemap as `layout` says; existing files are replaced. Everything is decoded before
    /// anything is written, and each file is written under a temporary name first.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::OutputIsInput`] when a file to write is the header or one of
    /// the parts, [`ConversionError::Write`] when a file cannot be written, a
    /// [`ConversionError`] wrapping [`TextureError::UnsupportedImage`] for a volume, a texture
    /// array or a cubemap array, and the errors of [`Texture::decode_surfaces_hdr`].
    pub fn exr_to_file_as(
        &self,
        out: &Path,
        level: u32,
        layout: CubemapLayout,
    ) -> Result<ExrReport, ConversionError> {
        let cube = is_cube(self.header());
        let files: Vec<PathBuf> = if cube && layout == CubemapLayout::Faces {
            CUBE_FACE_NAMES
                .iter()
                .map(|face| face_path(out, face))
                .collect()
        } else {
            vec![out.to_path_buf()]
        };
        for file in &files {
            self.refuse_input(file)?;
        }
        let images = match (cube, layout) {
            (false, _) => vec![self.decode_mip_hdr(level)?],
            (true, CubemapLayout::Faces) => self.decode_surfaces_hdr(level)?,
            (true, CubemapLayout::Cross) => vec![
                cube_cross_hdr(&self.decode_surfaces_hdr(level)?)
                    .map_err(|error| ConversionError::of_texture(self.name(), error))?,
            ],
        };
        files
            .par_iter()
            .zip(&images)
            .try_for_each(|(file, image)| {
                write_replacing(file, |writer| {
                    write_exr(image, writer)
                        .map_err(|error| ConversionError::of_texture(self.name(), error))
                })
            })?;
        let (width, height) = images
            .first()
            .map_or((0, 0), |image| (image.width(), image.height()));
        Ok(ExrReport {
            level,
            width,
            height,
            files,
        })
    }

    fn check_high_range(&self) -> Result<(), ConversionError> {
        let format = self.header().format();
        if format.is_high_range() {
            Ok(())
        } else {
            Err(ConversionError::of_texture(
                self.name(),
                TextureError::NotHighRange(format),
            ))
        }
    }
}
