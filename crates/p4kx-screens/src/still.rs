// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;
use std::io::Write;
use std::path::Path;

use p4kx_dataforge::DataForge;
use p4kx_extraction::{ConversionError, write_replacing};
use p4kx_gfx::{Localise, RasterOptions, RgbaImage, Skipped};
use thiserror::Error;

use crate::compose::{ScreenComposer, TextSources};
use crate::error::ScreenError;
use crate::report::Report;

mod texts;

pub use texts::ScreenTexts;

/// The file format a [`Still`] is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StillFormat {
    /// An 8-bit RGBA PNG.
    Png,
    /// An SVG document.
    Svg,
}

/// What [`Still::render`] draws: one canvas, how, and in which colours.
#[derive(Debug, Clone, Copy)]
pub struct StillRequest<'a> {
    /// The canvas to draw, a GUID or a record path.
    pub screen: &'a str,
    /// The manufacturer code whose colour style the canvas is drawn with.
    pub brand: Option<&'a str>,
    /// The size of the image and what lies under it.
    pub raster: RasterOptions,
    /// The format the still is made for.
    pub format: StillFormat,
}

/// A failure to write a [`Still`].
#[derive(Debug, Error)]
pub enum StillWriteError {
    /// The still could not be encoded.
    #[error(transparent)]
    Encode(#[from] ScreenError),
    /// The file could not be written.
    #[error(transparent)]
    Write(#[from] ConversionError),
}

#[derive(Debug)]
enum Picture {
    Png(RgbaImage),
    Svg(Vec<u8>),
}

/// A screen drawn as an image or an SVG document, with what the drawing left out.
#[derive(Debug)]
pub struct Still {
    picture: Picture,
    width: u32,
    height: u32,
    report: Report,
    skipped: Vec<Skipped>,
    warnings: Vec<String>,
}

impl Still {
    /// Composes the canvas of `request` of `forge` and draws it in the format of the request,
    /// the texts with `texts`.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`ScreenComposer::compose`], and [`ScreenError::Gfx`] when the size
    /// of the request cannot be drawn.
    pub fn render(
        forge: &DataForge<'_>,
        texts: &ScreenTexts,
        request: &StillRequest<'_>,
    ) -> Result<Self, ScreenError> {
        let mut warnings = Vec::new();
        let ini = texts.parsed(&mut warnings);
        let lookup = |key: &str| ini.as_ref()?.resolve(key).map(texts::decoded);
        let mut composer = ScreenComposer::new(forge);
        if let Some(fonts) = texts.fonts() {
            composer = composer.with_texts(TextSources {
                fonts,
                localise: ini.as_ref().map(|_| &lookup as Localise<'_>),
            });
        }
        let scene = composer.compose(request.screen, request.brand)?;
        let (picture, width, height, skipped) = match request.format {
            StillFormat::Png => {
                let raster = scene.rasterize(&request.raster)?;
                let (width, height) = (raster.image.width(), raster.image.height());
                (Picture::Png(raster.image), width, height, raster.skipped)
            }
            StillFormat::Svg => {
                let mut bytes = Vec::new();
                let written =
                    scene.write_svg(&request.raster, Some(request.screen), &mut bytes)?;
                (
                    Picture::Svg(bytes),
                    written.width,
                    written.height,
                    written.skipped,
                )
            }
        };
        Ok(Self {
            picture,
            width,
            height,
            report: scene.report().clone(),
            skipped,
            warnings,
        })
    }

    /// Returns the format the still was drawn in.
    #[must_use]
    pub const fn format(&self) -> StillFormat {
        match self.picture {
            Picture::Png(_) => StillFormat::Png,
            Picture::Svg(_) => StillFormat::Svg,
        }
    }

    /// Returns the width of the image or document in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height of the image or document in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the pixels of a PNG still, with straight alpha; `None` for an SVG one.
    #[must_use]
    pub const fn image(&self) -> Option<&RgbaImage> {
        match &self.picture {
            Picture::Png(image) => Some(image),
            Picture::Svg(_) => None,
        }
    }

    /// Returns what the composition drew and left out.
    #[must_use]
    pub const fn report(&self) -> &Report {
        &self.report
    }

    /// Returns the elements the renderer skipped or drew without an effect, in drawing order.
    #[must_use]
    pub fn skipped(&self) -> &[Skipped] {
        &self.skipped
    }

    /// Returns why texts are drawn without something they need: the texts of the language could
    /// not be parsed.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Returns whether the whole screen was drawn with every effect.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.report.is_complete() && self.skipped.is_empty()
    }

    /// Returns the bytes of the file: the PNG encoded, or the SVG document.
    ///
    /// # Errors
    ///
    /// Returns [`ScreenError::Gfx`] when the PNG cannot be encoded.
    pub fn encode(&self) -> Result<Cow<'_, [u8]>, ScreenError> {
        match &self.picture {
            Picture::Svg(bytes) => Ok(Cow::Borrowed(bytes)),
            Picture::Png(image) => {
                let mut bytes = Vec::new();
                image.write_png(&mut bytes)?;
                Ok(Cow::Owned(bytes))
            }
        }
    }

    /// Writes the file to `out`, replacing it; it is written whole or not at all.
    ///
    /// # Errors
    ///
    /// Returns [`StillWriteError::Encode`] when the PNG cannot be encoded and
    /// [`StillWriteError::Write`] when the file cannot be written.
    pub fn write(&self, out: &Path) -> Result<(), StillWriteError> {
        let bytes = self.encode()?;
        write_replacing(out, |writer| {
            writer
                .write_all(&bytes)
                .map_err(|source| ConversionError::Write {
                    path: out.to_owned(),
                    source,
                })
        })?;
        Ok(())
    }
}
