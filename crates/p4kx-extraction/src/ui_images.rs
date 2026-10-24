// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io;
use std::sync::OnceLock;

use p4kx_archive::Archive;
use p4kx_texture::{DdsHeader, RgbaImage, TextureError};

use crate::path_index::{PathIndex, PathKeys};
use crate::sound::read_capped;
use crate::{ConversionError, Texture, UnknownEntries};

/// The most pixels of a texture decoded for a screen, as in `p4kx-gfx`.
pub const MAX_SCREEN_TEXTURE_PIXELS: u64 = 1 << 25;

/// Reads the textures and vector graphics that the `BuildingBlocks` screens of an Archive draw.
///
/// Paths are those of the Records, with `/` or `\` and in any case; an Entry is looked up with
/// and without the `Data/` folder the Archive keeps game files in. The Entries are indexed by path
/// on the first read.
#[derive(Debug)]
pub struct ArchiveImages<'a> {
    archive: &'a Archive,
    keys: OnceLock<PathKeys>,
}

impl<'a> ArchiveImages<'a> {
    /// Reads textures and vector graphics from `archive`.
    #[must_use]
    pub const fn new(archive: &'a Archive) -> Self {
        Self {
            archive,
            keys: OnceLock::new(),
        }
    }

    fn index(&self) -> PathIndex<'_> {
        self.keys
            .get_or_init(|| PathKeys::new(self.archive))
            .index(self.archive)
    }

    fn locate(index: &PathIndex<'_>, path: &str) -> Option<String> {
        let under_data = format!("Data/{path}");
        [under_data.as_str(), path]
            .into_iter()
            .find_map(|candidate| index.find(candidate))
            .map(|entry| entry.path().to_owned())
    }

    /// Reads the texture at `path` as 8-bit RGBA, with its alpha tail merged in.
    ///
    /// The mip chosen is the smallest one whose longer side is at least `wanted_side`, so a
    /// texture is not decoded larger than the screen it is drawn on, nor larger than
    /// [`MAX_SCREEN_TEXTURE_PIXELS`] pixels.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::UnknownEntry`] when the Archive has no such texture, and
    /// another [`ConversionError`] when it cannot be read, decoded or is too large in every mip
    /// or holds several images.
    pub fn texture(&self, path: &str, wanted_side: u32) -> Result<RgbaImage, ConversionError> {
        let index = self.index();
        let header_path =
            Self::locate(&index, path).ok_or_else(|| UnknownEntries(vec![path.to_owned()]))?;
        let texture = Texture::from_index(&index, &header_path)?;
        let level = Self::level(texture.header(), wanted_side).ok_or_else(|| {
            ConversionError::of_texture(
                &header_path,
                TextureError::InvalidHeader("the texture is too large for a screen"),
            )
        })?;
        texture.decode_mip(level)
    }

    fn level(header: &DdsHeader, wanted_side: u32) -> Option<u32> {
        let side = |level: u32| {
            (
                header.width().checked_shr(level).unwrap_or(0).max(1),
                header.height().checked_shr(level).unwrap_or(0).max(1),
            )
        };
        let last = header.mip_count().saturating_sub(1);
        let wanted = wanted_side.max(1);
        let level = (0..=last)
            .take_while(|level| {
                let (width, height) = side(*level);
                width.max(height) >= wanted
            })
            .last()
            .unwrap_or(0);
        (level..=last).find(|level| {
            let (width, height) = side(*level);
            u64::from(width) * u64::from(height) <= MAX_SCREEN_TEXTURE_PIXELS
        })
    }

    /// Reads the bytes of the vector graphic at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::UnknownEntry`] when the Archive has no such file,
    /// [`ConversionError::ReadEntry`] when it cannot be read or holds more than `limit` bytes.
    pub fn vector(&self, path: &str, limit: u64) -> Result<Vec<u8>, ConversionError> {
        let index = self.index();
        let name =
            Self::locate(&index, path).ok_or_else(|| UnknownEntries(vec![path.to_owned()]))?;
        let entry = index
            .find(&name)
            .ok_or_else(|| UnknownEntries(vec![path.to_owned()]))?;
        let read = |source| ConversionError::ReadEntry {
            entry: name.clone(),
            source,
        };
        let reader = self.archive.open_entry(entry)?;
        read_capped(reader, entry.uncompressed_size(), limit)
            .map_err(read)?
            .ok_or_else(|| {
                read(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("the file has more than {limit} bytes"),
                ))
            })
    }
}
