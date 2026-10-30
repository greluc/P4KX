// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;

use p4kx_dataforge::DataForge;
use p4kx_gfx::{RasterOptions, RgbaImage, Skipped};

use crate::compose::{ScreenComposer, TextSources};
use crate::error::ScreenError;
use crate::images::ImageSources;
use crate::report::Report;
use crate::variables::Variable;

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
    /// The game variables the bindings are evaluated with instead of zero.
    pub variables: &'a [Variable],
    /// The size of the image and what lies under it.
    pub raster: RasterOptions,
    /// The format the still is made for.
    pub format: StillFormat,
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
}

impl Still {
    /// Composes the canvas of `request` of `forge` and draws it in the format of the request,
    /// the texts with `texts` and the textures and vector graphics with `images`.
    ///
    /// Widgets that need a source that is `None` are listed in the report.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`ScreenComposer::compose`], and [`ScreenError::Gfx`] when the size
    /// of the request cannot be drawn.
    pub fn render(
        forge: &DataForge<'_>,
        texts: Option<TextSources<'_>>,
        images: Option<ImageSources<'_>>,
        request: &StillRequest<'_>,
    ) -> Result<Self, ScreenError> {
        Self::render_while(forge, texts, images, request, &|| false)
    }

    /// Like [`Still::render`], asking `stopped` before the composition, after it and before the
    /// rasterisation.
    ///
    /// # Errors
    ///
    /// Returns [`ScreenError::Stopped`] when `stopped` returned `true`, and the errors of
    /// [`Still::render`].
    pub fn render_while(
        forge: &DataForge<'_>,
        texts: Option<TextSources<'_>>,
        images: Option<ImageSources<'_>>,
        request: &StillRequest<'_>,
        stopped: &dyn Fn() -> bool,
    ) -> Result<Self, ScreenError> {
        if stopped() {
            return Err(ScreenError::Stopped);
        }
        let mut composer = ScreenComposer::new(forge).with_variables(request.variables);
        if let Some(images) = images {
            composer = composer.with_images(images);
        }
        if let Some(texts) = texts {
            composer = composer.with_texts(texts);
        }
        let scene = composer.compose(request.screen, request.brand)?;
        if stopped() {
            return Err(ScreenError::Stopped);
        }
        let (picture, width, height, skipped) = match request.format {
            StillFormat::Png => {
                let raster = scene.rasterize(&request.raster)?;
                let (width, height) = (raster.image.width(), raster.image.height());
                (Picture::Png(raster.image), width, height, raster.skipped)
            }
            StillFormat::Svg => {
                let mut bytes = Vec::new();
                let written = scene.write_svg(&request.raster, Some(request.screen), &mut bytes)?;
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
}
