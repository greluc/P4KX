// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::GfxError;
use crate::assets::Assets;
use crate::header::{Header, read_file_header, read_movie};
use crate::scene::Scene;
use crate::scene_compose::Composer;
use crate::text_fonts::TextOptions;
use crate::walker::Walker;

/// A GFx or SWF movie, decompressed in memory.
#[derive(Debug, Clone)]
pub struct Movie {
    header: Header,
    data: Vec<u8>,
    tags: usize,
}

impl Movie {
    /// Reads the header of the movie in `bytes` and decompresses it.
    ///
    /// The tags are only read by [`Movie::assets`].
    ///
    /// # Errors
    ///
    /// Returns [`GfxError::Signature`] when `bytes` is no GFx or SWF movie,
    /// [`GfxError::TooLarge`] when it decompresses to more than [`crate::MAX_MOVIE_LEN`],
    /// [`GfxError::Decompress`] when its compressed data is corrupt, and another [`GfxError`]
    /// when its header is truncated or invalid.
    pub fn parse(bytes: Vec<u8>) -> Result<Self, GfxError> {
        let (header, data, tags) = read_movie(bytes)?;
        Ok(Self { header, data, tags })
    }

    /// Reads the uncompressed length that the file header of `bytes` declares, file header
    /// included, without decompressing anything.
    ///
    /// # Errors
    ///
    /// Returns [`GfxError::Signature`] when `bytes` is no GFx or SWF movie,
    /// [`GfxError::DeclaredLength`] when the length is shorter than the file header, and another
    /// [`GfxError`] when the file header is truncated.
    pub fn declared_len(bytes: &[u8]) -> Result<u32, GfxError> {
        read_file_header(bytes).map(|(_, _, declared)| declared)
    }

    /// Returns the header.
    #[must_use]
    pub const fn header(&self) -> &Header {
        &self.header
    }

    /// Returns the length of the uncompressed movie, file header included.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns whether the uncompressed movie holds no bytes, which a parsed movie never does.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Reads every tag, sprites included, into the definitions the movie holds.
    #[must_use]
    pub fn assets(&self) -> Assets<'_> {
        Walker::read(self.data.get(self.tags..).unwrap_or_default(), self.tags)
    }

    /// Composes the first frame of the main timeline from `assets`, read from this movie, into
    /// a scene on its stage.
    ///
    /// Placements that cannot be drawn are skipped and listed in [`Scene::skipped`].
    #[must_use]
    pub fn first_frame(&self, assets: &Assets<'_>) -> Scene {
        self.first_frame_with_text(assets, &TextOptions::default())
    }

    /// Composes the first frame as [`Movie::first_frame`] does, with the fonts and the
    /// localisation of `options` for its texts.
    ///
    /// Static texts and text fields become glyph shapes of the scene, so every backend draws
    /// them alike. A character whose font or glyph is missing is drawn as an empty box and listed
    /// in [`Scene::skipped`]; no font of the system is used.
    #[must_use]
    pub fn first_frame_with_text(&self, assets: &Assets<'_>, options: &TextOptions<'_>) -> Scene {
        Composer::compose(assets, self.header.frame_size, options)
    }
}
