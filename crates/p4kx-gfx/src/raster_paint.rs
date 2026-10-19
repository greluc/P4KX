use std::collections::HashMap;

use tiny_skia::{
    Color, ColorU8, FilterQuality, GradientStop, IntSize, LineCap, LineJoin, LinearGradient, Paint,
    Path, PathBuilder, Pattern, Pixmap, Point, RadialGradient, Shader, SpreadMode, Stroke,
    Transform,
};

use crate::geometry::Rgba;
use crate::image::RgbaImage;
use crate::place::ColorTransform;
use crate::scene::{
    Affine, BitmapPaint, Cap, GradientKind, GradientPaint, Join, Paint as ScenePaint, PathCommand,
    ScenePath, Spread, StrokeStyle,
};
use crate::scene_report::SkipReason;

const GRADIENT_EXTENT: f32 = 16_384.0;

/// The bitmaps of a rasterisation as premultiplied pixmaps, each converted once, and their
/// colour-transformed copies; the pixels of all copies together stay within a limit.
pub(crate) struct BitmapCache<'b> {
    bitmaps: &'b HashMap<u16, RgbaImage>,
    plain: HashMap<u16, Option<Pixmap>>,
    transformed: HashMap<(u16, [u32; 8]), Option<Pixmap>>,
    copied_pixels: u64,
    max_pixels: u64,
}

impl<'b> BitmapCache<'b> {
    /// Starts an empty cache over the decoded `bitmaps` whose copies may hold `max_pixels`
    /// pixels together.
    pub(crate) fn new(bitmaps: &'b HashMap<u16, RgbaImage>, max_pixels: u64) -> Self {
        Self {
            bitmaps,
            plain: HashMap::new(),
            transformed: HashMap::new(),
            copied_pixels: 0,
            max_pixels,
        }
    }

    fn charge(&mut self, image: &RgbaImage) -> Result<(), SkipReason> {
        let pixels = u64::from(image.width()) * u64::from(image.height());
        let total = self.copied_pixels.saturating_add(pixels);
        if total > self.max_pixels {
            return Err(SkipReason::BitmapMemory);
        }
        self.copied_pixels = total;
        Ok(())
    }

    fn pixmap(&mut self, id: u16, transform: &ColorTransform) -> Result<&Pixmap, SkipReason> {
        let missing = SkipReason::MissingBitmap { bitmap_id: id };
        let image = self.bitmaps.get(&id).ok_or(missing)?;
        if transform.is_identity() {
            if !self.plain.contains_key(&id) {
                self.charge(image)?;
                self.plain.insert(id, premultiply(image, None));
            }
            return self.plain.get(&id).and_then(Option::as_ref).ok_or(missing);
        }
        let key = (id, key(transform));
        if !self.transformed.contains_key(&key) {
            self.charge(image)?;
            self.transformed
                .insert(key, premultiply(image, Some(transform)));
        }
        self.transformed
            .get(&key)
            .and_then(Option::as_ref)
            .ok_or(missing)
    }
}

fn key(transform: &ColorTransform) -> [u32; 8] {
    let mut key = [0; 8];
    for (slot, value) in key
        .iter_mut()
        .zip(transform.multiply.iter().chain(&transform.add))
    {
        *slot = value.to_bits();
    }
    key
}

fn premultiply(image: &RgbaImage, transform: Option<&ColorTransform>) -> Option<Pixmap> {
    let mut data = Vec::with_capacity(image.pixels().len());
    let (pixels, _) = image.pixels().as_chunks::<4>();
    for &[red, green, blue, alpha] in pixels {
        let mut color = Rgba {
            red,
            green,
            blue,
            alpha,
        };
        if let Some(transform) = transform {
            color = transform.apply(color);
        }
        let premultiplied =
            ColorU8::from_rgba(color.red, color.green, color.blue, color.alpha).premultiply();
        data.extend_from_slice(&[
            premultiplied.red(),
            premultiplied.green(),
            premultiplied.blue(),
            premultiplied.alpha(),
        ]);
    }
    Pixmap::from_vec(data, IntSize::from_wh(image.width(), image.height())?)
}

/// Builds the tiny-skia paint of `paint` with its colours transformed.
pub(crate) fn paint<'c>(
    cache: &'c mut BitmapCache<'_>,
    color_transform: &ColorTransform,
    paint: &ScenePaint,
) -> Result<Paint<'c>, SkipReason> {
    let shader = match paint {
        ScenePaint::Solid(color) => Shader::SolidColor(color_of(color_transform.apply(*color))),
        ScenePaint::Gradient(gradient) => gradient_shader(gradient, color_transform)?,
        ScenePaint::Bitmap(bitmap) => bitmap_shader(cache, bitmap, color_transform)?,
    };
    Ok(Paint {
        shader,
        anti_alias: true,
        ..Paint::default()
    })
}

fn color_of(color: Rgba) -> Color {
    Color::from_rgba8(color.red, color.green, color.blue, color.alpha)
}

fn gradient_shader(
    gradient: &GradientPaint,
    color_transform: &ColorTransform,
) -> Result<Shader<'static>, SkipReason> {
    let stops: Vec<GradientStop> = gradient
        .stops
        .iter()
        .map(|stop| {
            GradientStop::new(
                f32::from(stop.ratio) / 255.0,
                color_of(color_transform.apply(stop.color)),
            )
        })
        .collect();
    let mode = match gradient.spread {
        Spread::Pad => SpreadMode::Pad,
        Spread::Reflect => SpreadMode::Reflect,
        Spread::Repeat => SpreadMode::Repeat,
    };
    let to_shape = transform(&gradient.transform).ok_or(SkipReason::InvalidGeometry)?;
    let shader = match gradient.kind {
        GradientKind::Linear => LinearGradient::new(
            Point::from_xy(-GRADIENT_EXTENT, 0.0),
            Point::from_xy(GRADIENT_EXTENT, 0.0),
            stops,
            mode,
            to_shape,
        ),
        GradientKind::Radial | GradientKind::Focal(_) => {
            let focal = match gradient.kind {
                GradientKind::Focal(focal) => focal * GRADIENT_EXTENT,
                _ => 0.0,
            };
            RadialGradient::new(
                Point::from_xy(focal, 0.0),
                0.0,
                Point::from_xy(0.0, 0.0),
                GRADIENT_EXTENT,
                stops,
                mode,
                to_shape,
            )
        }
    };
    shader.ok_or(SkipReason::InvalidGeometry)
}

fn bitmap_shader<'c>(
    cache: &'c mut BitmapCache<'_>,
    bitmap: &BitmapPaint,
    color_transform: &ColorTransform,
) -> Result<Shader<'c>, SkipReason> {
    let to_shape = transform(&bitmap.transform).ok_or(SkipReason::InvalidGeometry)?;
    let (pixmap, opacity) = match opacity_only(color_transform) {
        Some(opacity) => (
            cache.pixmap(bitmap.bitmap_id, &ColorTransform::IDENTITY)?,
            opacity,
        ),
        None => (cache.pixmap(bitmap.bitmap_id, color_transform)?, 1.0),
    };
    Ok(Pattern::new(
        pixmap.as_ref(),
        if bitmap.repeating {
            SpreadMode::Repeat
        } else {
            SpreadMode::Pad
        },
        if bitmap.smoothed {
            FilterQuality::Bilinear
        } else {
            FilterQuality::Nearest
        },
        opacity,
        to_shape,
    ))
}

/// Returns the opacity a colour transform comes down to when it only scales alpha within 0–1.
fn opacity_only(transform: &ColorTransform) -> Option<f32> {
    let [red, green, blue, alpha] = transform.multiply;
    let unchanged = (red - 1.0).abs() < f32::EPSILON
        && (green - 1.0).abs() < f32::EPSILON
        && (blue - 1.0).abs() < f32::EPSILON
        && transform.add.iter().all(|add| add.abs() < f32::EPSILON);
    (unchanged && (0.0..=1.0).contains(&alpha)).then_some(alpha)
}

/// Converts an affine transform, `None` when a factor is not finite.
pub(crate) fn transform(affine: &Affine) -> Option<Transform> {
    affine
        .is_finite()
        .then(|| Transform::from_row(affine.a, affine.b, affine.c, affine.d, affine.tx, affine.ty))
}

/// Converts a path, `None` when it is empty or not finite.
pub(crate) fn path(scene_path: &ScenePath) -> Option<Path> {
    let mut builder = PathBuilder::with_capacity(scene_path.commands.len(), 0);
    for command in &scene_path.commands {
        match *command {
            PathCommand::MoveTo(point) => builder.move_to(point.x, point.y),
            PathCommand::LineTo(point) => builder.line_to(point.x, point.y),
            PathCommand::QuadTo(control, point) => {
                builder.quad_to(control.x, control.y, point.x, point.y);
            }
            PathCommand::Close => builder.close(),
        }
    }
    builder.finish()
}

/// Converts a stroke style.
pub(crate) const fn stroke(style: &StrokeStyle) -> Stroke {
    Stroke {
        width: style.width.max(0.0),
        miter_limit: style.miter_limit,
        line_cap: match style.cap {
            Cap::Round => LineCap::Round,
            Cap::Butt => LineCap::Butt,
            Cap::Square => LineCap::Square,
        },
        line_join: match style.join {
            Join::Round => LineJoin::Round,
            Join::Bevel => LineJoin::Bevel,
            Join::Miter => LineJoin::Miter,
        },
        dash: None,
    }
}
