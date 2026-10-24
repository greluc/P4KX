// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_texture::{RgbaImage, cube_cross};

use crate::texture_job::{Member, Role, role};
use crate::{ConversionError, Texture};

const CUBE_FACES: u32 = 6;

impl Texture<'_> {
    /// Returns whether the Entry at `path` is the header of a texture or of an alpha tail, the
    /// Entries a texture is opened from; its parts `.dds.N` and `.dds.Na` are not.
    #[must_use]
    pub fn is_header_path(path: &str) -> bool {
        matches!(
            role(path),
            Some(Role::Header | Role::Member(_, Member::AlphaHeader))
        )
    }

    /// Reads and decodes mip `level` as the one image a preview shows: the mip itself with the
    /// alpha tail merged in, or a cubemap's six faces laid out as a horizontal cross.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Texture::decode_surfaces`], and a [`ConversionError`] wrapping a
    /// texture error when the texture holds several images that are not one cubemap or the cross
    /// cannot be laid out.
    pub fn decode_preview(&self, level: u32) -> Result<RgbaImage, ConversionError> {
        let header = self.header();
        if header.faces() == CUBE_FACES && header.array_size() == 1 {
            let faces = self.decode_surfaces(level)?;
            return cube_cross(&faces)
                .map_err(|error| ConversionError::of_texture(self.name(), error));
        }
        self.decode_mip(level)
    }
}
