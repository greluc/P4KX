use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use p4kx_gfx::{
    Assets, BitmapSource, GfxError, MAX_BITMAP_PIXELS, RasterOptions, RgbaImage, Skipped,
    TagFailure,
};
use rayon::prelude::*;

use crate::output_file::{write_error, write_replacing};
use crate::pixel_budget::PixelBudget;
use crate::texture_parts::is_same_file;
use crate::ui_movie::Origin;
use crate::{ConversionError, UiMovie};

/// The pixels the bitmaps of one rendered frame may hold together, and the pixels its parallel
/// bitmap decodes may hold at the same time.
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
    /// The bitmaps that could not be decoded, ascending by ID; their fills are skipped.
    pub bitmap_failures: Vec<UiBitmapFailure>,
    /// The tags of the movie that could not be read, whose definitions are therefore missing.
    pub tag_failures: Vec<TagFailure>,
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
    /// decodes them, external images and sub-images included, and hold at most
    /// [`MAX_UI_RENDER_BITMAP_PIXELS`] together. Elements that cannot be drawn are skipped and
    /// listed in the result.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::UiMovie`] when the stage has no area and
    /// [`ConversionError::UiCanvas`] when the canvas is empty or too large.
    pub fn render(&self, options: &RasterOptions) -> Result<UiRender, ConversionError> {
        let mut assets = self.movie().assets();
        let scene = self.movie().first_frame(&assets);
        let budget = PixelBudget::new(MAX_UI_RENDER_BITMAP_PIXELS);
        let held = AtomicU64::new(0);
        let decoded: Vec<(u16, Result<RgbaImage, ConversionError>)> = scene
            .bitmap_ids()
            .par_iter()
            .map(|&id| (id, self.frame_bitmap(&assets, id, &budget, &held)))
            .collect();
        let mut bitmaps = HashMap::with_capacity(decoded.len());
        let mut bitmap_failures = Vec::new();
        for (id, result) in decoded {
            match result {
                Ok(image) => {
                    bitmaps.insert(id, image);
                }
                Err(error) => bitmap_failures.push(UiBitmapFailure { id, error }),
            }
        }
        let raster = scene
            .rasterize(&bitmaps, options)
            .map_err(|source| match source {
                GfxError::CanvasSize { .. } => ConversionError::UiCanvas { source },
                source => ConversionError::UiMovie {
                    movie: self.name().to_owned(),
                    source,
                },
            })?;
        Ok(UiRender {
            image: raster.image,
            skipped: raster.skipped,
            bitmap_failures,
            tag_failures: std::mem::take(&mut assets.failures),
        })
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
        if let Origin::File(file) = self.origin()
            && is_same_file(out, [file.clone()])
        {
            return Err(ConversionError::OutputIsInput {
                path: out.to_path_buf(),
            });
        }
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

    fn frame_bitmap(
        &self,
        assets: &Assets<'_>,
        id: u16,
        budget: &PixelBudget,
        held: &AtomicU64,
    ) -> Result<RgbaImage, ConversionError> {
        let bitmap = assets
            .bitmaps
            .iter()
            .find(|bitmap| bitmap.id == id)
            .ok_or_else(|| ConversionError::UiBitmapMissing {
                movie: self.name().to_owned(),
                id,
            })?;
        let image = if let BitmapSource::SubImage(sub) = &bitmap.source {
            let base = assets
                .bitmaps
                .iter()
                .find(|base| base.id == sub.image_id)
                .ok_or_else(|| self.cut_error(bitmap))?;
            let (image, _lease) = self.decode_bitmap(assets, base, budget)?;
            image
                .crop(
                    (u32::from(sub.x1), u32::from(sub.y1)),
                    (u32::from(sub.x2), u32::from(sub.y2)),
                )
                .ok_or_else(|| self.cut_error(bitmap))?
        } else {
            self.decode_bitmap(assets, bitmap, budget)?.0
        };
        let pixels = u64::from(image.width()) * u64::from(image.height());
        let before = held.fetch_add(pixels, Ordering::SeqCst);
        if before.saturating_add(pixels) > MAX_UI_RENDER_BITMAP_PIXELS {
            held.fetch_sub(pixels, Ordering::SeqCst);
            return Err(ConversionError::UiRenderBitmaps {
                movie: self.name().to_owned(),
                id,
                limit: MAX_UI_RENDER_BITMAP_PIXELS,
            });
        }
        Ok(image)
    }
}
