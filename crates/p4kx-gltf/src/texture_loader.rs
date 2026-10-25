// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_texture::RgbaImage;
use thiserror::Error;

/// Why a texture could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum TextureLoadError {
    /// No such texture exists.
    #[error("the texture does not exist")]
    NotFound,
    /// The texture exists but cannot be read, decoded or used.
    #[error("{0}")]
    Unreadable(String),
}

/// A decoded texture.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LoadedTexture {
    /// The pixels, 8-bit RGBA, any alpha tail merged into the alpha channel.
    pub image: RgbaImage,
    /// Whether the alpha channel holds data, from an alpha tail or from the texture's own format,
    /// rather than the opaque value a format without alpha decodes to.
    pub has_alpha: bool,
}

impl LoadedTexture {
    /// Creates a texture from `image`, whose alpha channel holds data when `has_alpha` is set.
    #[must_use]
    pub const fn new(image: RgbaImage, has_alpha: bool) -> Self {
        Self { image, has_alpha }
    }
}

/// Gives the GLB writer the textures of the materials, wherever they are stored.
pub trait TextureLoader: Sync {
    /// Loads mip `mip`, 0 being the largest, of the texture at `path`. A `mip` beyond the
    /// texture's last one selects the last.
    ///
    /// `path` is relative to the `Data` folder, with `/` as separator and the extension `.dds`; it
    /// has no empty, `.` or `..` component.
    ///
    /// # Errors
    ///
    /// Returns [`TextureLoadError::NotFound`] when there is no such texture and
    /// [`TextureLoadError::Unreadable`] when it cannot be used.
    fn load(&self, path: &str, mip: u32) -> Result<LoadedTexture, TextureLoadError>;

    /// Returns the width and height [`TextureLoader::load`] would give for mip `mip` of the
    /// texture at `path`, without decoding it, or `None` when the loader cannot tell that
    /// cheaply. The writer uses it to leave out an image that would exceed the budget before
    /// decoding anything.
    ///
    /// # Errors
    ///
    /// Returns the errors [`TextureLoader::load`] would return for the same texture.
    fn dimensions(&self, _path: &str, _mip: u32) -> Result<Option<(u32, u32)>, TextureLoadError> {
        Ok(None)
    }
}

/// A loader without textures: every texture is unreadable.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct NoTextures;

impl TextureLoader for NoTextures {
    fn load(&self, _path: &str, _mip: u32) -> Result<LoadedTexture, TextureLoadError> {
        Err(TextureLoadError::Unreadable(
            "no texture source was given".to_owned(),
        ))
    }
}
