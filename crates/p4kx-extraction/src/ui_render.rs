use std::path::Path;

use p4kx_gfx::{GfxError, MAX_BITMAP_PIXELS, RasterOptions, RgbaImage, Scene, Skipped, TagFailure};

use crate::output_file::{write_error, write_replacing};
use crate::texture_parts::is_same_file;
use crate::ui_movie::Origin;
use crate::ui_render_bitmaps::FrameBitmaps;
use crate::{ConversionError, UiMovie};

/// The pixels the decoded bitmaps of one rendered frame may hold together, granted in ascending
/// ID order, and the pixels its parallel bitmap decodes may hold at the same time.
pub const MAX_UI_RENDER_BITMAP_PIXELS: u64 = 2 * MAX_BITMAP_PIXELS;

/// A bitmap a rendered frame fills with that could not be decoded.
#[derive(Debug)]
pub struct UiBitmapFailure {
    /// The ID of the bitmap.
    pub id: u16,
    /// Why.
    pub error: ConversionError,
}

/// What [`UiMovie::render`] drew: the image and what it left out.
#[derive(Debug)]
pub struct UiRender {
    /// The image with straight alpha.
    pub image: RgbaImage,
    /// The elements skipped or drawn without an effect, in drawing order.
    pub skipped: Vec<Skipped>,
    /// The bitmaps that could not be decoded or kept, ascending by ID; their fills are skipped.
    pub bitmap_failures: Vec<UiBitmapFailure>,
    /// The tags of the movie that could not be read, whose definitions are therefore missing.
    pub tag_failures: Vec<TagFailure>,
}

/// A composed first frame with its decoded bitmaps.
pub(crate) struct ComposedFrame {
    pub(crate) scene: Scene,
    pub(crate) bitmaps: FrameBitmaps,
    pub(crate) tag_failures: Vec<TagFailure>,
}

impl UiRender {
    /// Returns whether every tag was read and every element drawn with all its effects.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.skipped.is_empty() && self.bitmap_failures.is_empty() && self.tag_failures.is_empty()
    }
}

impl UiMovie<'_> {
    /// Renders the first frame of the main timeline, with its sprites, to an image.
    ///
    /// The bitmaps the frame fills with are decoded in parallel as [`UiMovie::extract_assets`]
    /// decodes them, external images and sub-images included, each cut bitmap once, and hold at
    /// most [`MAX_UI_RENDER_BITMAP_PIXELS`] together. Elements that cannot be drawn are skipped and
    /// listed in the result.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::UiMovie`] when the stage has no area and
    /// [`ConversionError::UiCanvas`] when the canvas is empty or too large.
    pub fn render(&self, options: &RasterOptions) -> Result<UiRender, ConversionError> {
        let frame = self.compose_frame(options)?;
        let raster = frame
            .scene
            .rasterize(&frame.bitmaps.images, options)
            .map_err(|source| self.frame_error(source))?;
        Ok(UiRender {
            image: raster.image,
            skipped: raster.skipped,
            bitmap_failures: frame.bitmaps.failures,
            tag_failures: frame.tag_failures,
        })
    }

    /// Composes the first frame and decodes the bitmaps it fills with, after checking that the
    /// canvas for `options` can be drawn.
    pub(crate) fn compose_frame(
        &self,
        options: &RasterOptions,
    ) -> Result<ComposedFrame, ConversionError> {
        let mut assets = self.movie().assets();
        let scene = self.movie().first_frame(&assets);
        scene
            .canvas_size(options.size)
            .map_err(|source| self.frame_error(source))?;
        let bitmaps = self.frame_bitmaps(&assets, &scene.bitmap_ids());
        Ok(ComposedFrame {
            scene,
            bitmaps,
            tag_failures: std::mem::take(&mut assets.failures),
        })
    }

    /// Refuses an output file that is the movie file itself.
    pub(crate) fn check_output(&self, out: &Path) -> Result<(), ConversionError> {
        if let Origin::File(file) = self.origin()
            && is_same_file(out, [file.clone()])
        {
            return Err(ConversionError::OutputIsInput {
                path: out.to_path_buf(),
            });
        }
        Ok(())
    }

    /// Wraps the error of drawing the frame.
    pub(crate) fn frame_error(&self, source: GfxError) -> ConversionError {
        match source {
            GfxError::CanvasSize { .. } => ConversionError::UiCanvas {
                movie: self.name().to_owned(),
                source,
            },
            source => ConversionError::UiMovie {
                movie: self.name().to_owned(),
                source,
            },
        }
    }

    /// Renders the first frame as [`UiMovie::render`] does and writes it to `out` as an 8-bit
    /// RGBA PNG, replacing the file.
    ///
    /// The PNG is written whole or not at all.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::OutputIsInput`] when `out` is the movie file, the errors of
    /// [`UiMovie::render`], and [`ConversionError::Write`] when the file cannot be written.
    pub fn render_to_file(
        &self,
        options: &RasterOptions,
        out: &Path,
    ) -> Result<UiRender, ConversionError> {
        self.check_output(out)?;
        let render = self.render(options)?;
        write_replacing(out, |writer| {
            render
                .image
                .write_png(writer)
                .map_err(|source| match source {
                    GfxError::Write(source) => write_error(out, source),
                    source => ConversionError::UiMovie {
                        movie: self.name().to_owned(),
                        source,
                    },
                })
        })?;
        Ok(render)
    }
}
