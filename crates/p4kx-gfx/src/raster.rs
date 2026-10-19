use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use tiny_skia::{Color, FillRule, Mask, Path, Pixmap};

use crate::GfxError;
use crate::geometry::Rgba;
use crate::image::RgbaImage;
use crate::raster_mask::Masks;
use crate::raster_paint::{BitmapCache, paint, path, stroke, transform};
use crate::scene::{Affine, LayerKind, Scene, SceneItem};
use crate::scene_report::{SkipReason, Skipped};

/// The widest and highest canvas, in pixels.
pub const MAX_CANVAS_SIDE: u32 = 16_384;

/// The most pixels a canvas may have.
pub const MAX_CANVAS_PIXELS: u64 = 1 << 25;

const TWIPS_PER_PIXEL: f32 = 20.0;

/// The size of a rendered frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CanvasSize {
    /// The stage size of the movie, one pixel per 20 twips.
    Stage,
    /// The given width; the height keeps the aspect ratio of the stage.
    Width(u32),
    /// The given width and height; the stage is scaled uniformly to fit and centred.
    Fit {
        /// The width in pixels.
        width: u32,
        /// The height in pixels.
        height: u32,
    },
}

/// What lies under a rasterised frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Background {
    /// Nothing.
    #[default]
    Transparent,
    /// The background colour the movie sets, transparency without one.
    Movie,
    /// This colour.
    Color(Rgba),
}

/// How a scene is rasterised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RasterOptions {
    /// The size of the image.
    pub size: CanvasSize,
    /// What lies under the frame.
    pub background: Background,
}

/// A rasterised frame and what of it was not drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster {
    /// The image with straight alpha.
    pub image: RgbaImage,
    /// The elements skipped by the scene and by the rasteriser.
    pub skipped: Vec<Skipped>,
}

impl Scene {
    /// Rasterises the scene with the decoded `bitmaps` its bitmap fills name, by ID.
    ///
    /// A layer whose bitmap is missing, whose paint or geometry cannot be drawn, or on which
    /// the rasteriser fails is skipped and listed in [`Raster::skipped`] after the elements the
    /// scene skipped.
    ///
    /// # Errors
    ///
    /// Returns [`GfxError::EmptyStage`] when the stage has no area and
    /// [`GfxError::CanvasSize`] when the canvas would be empty or larger than
    /// [`MAX_CANVAS_SIDE`] or [`MAX_CANVAS_PIXELS`].
    pub fn rasterize(
        &self,
        bitmaps: &HashMap<u16, RgbaImage>,
        options: &RasterOptions,
    ) -> Result<Raster, GfxError> {
        let (width, height, root) = canvas(self, options.size)?;
        let mut pixmap =
            Pixmap::new(width, height).ok_or(GfxError::CanvasSize { width, height })?;
        let background = match options.background {
            Background::Transparent => None,
            Background::Movie => self.background,
            Background::Color(color) => Some(color),
        };
        if let Some(color) = background {
            pixmap.fill(Color::from_rgba8(
                color.red,
                color.green,
                color.blue,
                color.alpha,
            ));
        }
        let paths: Vec<Vec<Option<Path>>> = self
            .shapes
            .iter()
            .map(|shape| shape.layers.iter().map(|layer| path(&layer.path)).collect())
            .collect();
        let mut frame = Frame {
            scene: self,
            paths: &paths,
            root,
            cache: BitmapCache::new(bitmaps),
            skipped: self.skipped.clone(),
        };
        let mut masks = Masks::new(self, width, height);
        for (at, item) in self.items.iter().enumerate() {
            let mask = match item.mask {
                Some(index) => match masks.get(self, &paths, index, &root) {
                    Ok(mask) => Some(mask),
                    Err(reason) => {
                        frame.skip(item, reason);
                        masks.release(at);
                        continue;
                    }
                },
                None => None,
            };
            frame.draw(item, &mut pixmap, mask);
            masks.release(at);
        }
        Ok(Raster {
            image: unpremultiply(&pixmap),
            skipped: frame.skipped,
        })
    }
}

/// The state of one rasterisation.
struct Frame<'s, 'b> {
    scene: &'s Scene,
    paths: &'s [Vec<Option<Path>>],
    root: Affine,
    cache: BitmapCache<'b>,
    skipped: Vec<Skipped>,
}

impl Frame<'_, '_> {
    fn skip(&mut self, item: &SceneItem, reason: SkipReason) {
        let entry = Skipped {
            id: item.origin.id,
            depth: item.origin.depth,
            sprite: item.origin.sprite,
            reason,
        };
        if self.skipped.last() != Some(&entry) {
            self.skipped.push(entry);
        }
    }

    fn draw(&mut self, item: &SceneItem, pixmap: &mut Pixmap, mask: Option<&Mask>) {
        let (Some(shape), Some(outlines)) = (
            self.scene.shapes.get(item.shape),
            self.paths.get(item.shape),
        ) else {
            return;
        };
        let Some(to_canvas) = transform(&self.root.then(&item.transform)) else {
            self.skip(item, SkipReason::InvalidGeometry);
            return;
        };
        for (layer, outline) in shape.layers.iter().zip(outlines) {
            let Some(outline) = outline else {
                continue;
            };
            let built = paint(&mut self.cache, &item.color_transform, &layer.paint);
            let drawn = match built {
                Ok(paint) => catch_unwind(AssertUnwindSafe(|| match &layer.kind {
                    LayerKind::Fill => {
                        pixmap.fill_path(outline, &paint, FillRule::EvenOdd, to_canvas, mask);
                    }
                    LayerKind::Stroke(style) => {
                        pixmap.stroke_path(outline, &paint, &stroke(style), to_canvas, mask);
                    }
                }))
                .map_err(|_| SkipReason::Raster),
                Err(reason) => Err(reason),
            };
            if let Err(reason) = drawn {
                self.skip(item, reason);
            }
        }
    }
}
fn canvas(scene: &Scene, size: CanvasSize) -> Result<(u32, u32, Affine), GfxError> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "stage sizes in twips fit an f32 closely enough"
    )]
    let (stage_width, stage_height) = (
        scene.stage.width() as f32 / TWIPS_PER_PIXEL,
        scene.stage.height() as f32 / TWIPS_PER_PIXEL,
    );
    if !(stage_width >= 1.0 && stage_height >= 1.0) {
        return Err(GfxError::EmptyStage);
    }
    let (width, height) = match size {
        CanvasSize::Stage => (pixels(stage_width), pixels(stage_height)),
        CanvasSize::Width(width) => (
            width,
            pixels(stage_height * pixel_count(width) / stage_width),
        ),
        CanvasSize::Fit { width, height } => (width, height),
    };
    let area = u64::from(width) * u64::from(height);
    if width == 0
        || height == 0
        || width > MAX_CANVAS_SIDE
        || height > MAX_CANVAS_SIDE
        || area > MAX_CANVAS_PIXELS
    {
        return Err(GfxError::CanvasSize { width, height });
    }
    let (canvas_width, canvas_height) = (pixel_count(width), pixel_count(height));
    let scale = (canvas_width / stage_width).min(canvas_height / stage_height) / TWIPS_PER_PIXEL;
    let offset_x = (stage_width * scale).mul_add(-TWIPS_PER_PIXEL, canvas_width) / 2.0;
    let offset_y = (stage_height * scale).mul_add(-TWIPS_PER_PIXEL, canvas_height) / 2.0;
    #[expect(
        clippy::cast_precision_loss,
        reason = "stage edges in twips fit an f32 closely enough"
    )]
    let (left, top) = (scene.stage.x_min as f32, scene.stage.y_min as f32);
    let root = Affine {
        a: scale,
        b: 0.0,
        c: 0.0,
        d: scale,
        tx: left.mul_add(-scale, offset_x),
        ty: top.mul_add(-scale, offset_y),
    };
    Ok((width, height, root))
}

const fn pixels(value: f32) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the value is clamped to the u32 range"
    )]
    let pixels = value.round().clamp(1.0, 4_294_967_040.0) as u32;
    pixels
}

#[expect(
    clippy::cast_precision_loss,
    reason = "canvas sides are at most MAX_CANVAS_SIDE"
)]
const fn pixel_count(value: u32) -> f32 {
    value as f32
}

fn unpremultiply(pixmap: &Pixmap) -> RgbaImage {
    let mut pixels = Vec::with_capacity(pixmap.data().len());
    for pixel in pixmap.pixels() {
        let color = pixel.demultiply();
        pixels.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }
    RgbaImage::new(pixmap.width(), pixmap.height(), pixels)
}
