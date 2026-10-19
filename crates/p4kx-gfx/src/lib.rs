//! Reads the GFx and SWF user-interface movies of Star Citizen Archives: the file and movie
//! header with zlib or LZMA compression, the tag stream with unknown tags kept, and the shapes,
//! morph shapes, bitmaps (embedded, GFx external and sub-images), fonts and sprites a movie
//! defines. Lossless and embedded bitmaps decode to RGBA and write as PNG; shapes serialise to
//! JSON in the units the file stores.
//!
//! The first frame of a movie composes into a [`Scene`] independent of any backend, which
//! [`Scene::rasterize`] draws with tiny-skia.
//!
//! Every size is checked against the data left and the limits [`MAX_MOVIE_LEN`],
//! [`MAX_BITMAP_PIXELS`], [`MAX_SHAPE_RECORDS`] and [`MAX_SPRITE_DEPTH`], a scene against
//! [`MAX_SCENE_PLACEMENTS`], [`MAX_SCENE_PATHS`], [`MAX_SCENE_SEGMENTS`], [`MAX_SCENE_RECORDS`],
//! [`MAX_SCENE_MASKS`] and [`MAX_GRADIENT_STOPS`], a canvas against [`MAX_CANVAS_SIDE`],
//! [`MAX_CANVAS_PIXELS`] and the fill area [`MAX_COVERAGE_FACTOR`] times its size;
//! corrupt input yields a [`GfxError`] or a skipped element, never a panic.

mod assets;
mod bitmap;
mod embedded_png;
mod error;
mod font;
mod font_info;
mod geometry;
mod header;
mod image;
mod jpeg;
mod json;
#[cfg(test)]
mod limit_tests;
mod line;
mod lossless;
mod morph;
mod morph_frame;
mod movie;
mod place;
mod raster;
mod raster_budget;
mod raster_mask;
mod raster_paint;
mod reader;
mod scene;
mod scene_compose;
mod scene_report;
mod scene_shape;
mod shape;
mod style;
mod tag;
mod timeline;
mod walker;

pub use assets::{Assets, OtherDefinition, TagCount, TagFailure, UnknownTag};
pub use bitmap::{
    Bitmap, BitmapSource, EmbeddedFormat, ExternalImage, ImageFileFormat, Jpeg, Lossless, SubImage,
};
pub use error::GfxError;
pub use font::{Font, FontFormat, FontLayout, Glyph, Kerning};
pub use geometry::{Matrix, Point, Rect, Rgba};
pub use header::{Compression, Header, MAX_MOVIE_LEN, Signature};
pub use image::{MAX_BITMAP_PIXELS, RgbaImage};
pub use morph::MorphShape;
pub use movie::Movie;
pub use place::{ColorTransform, Placement};
pub use raster::{
    Background, CanvasSize, MAX_CANVAS_PIXELS, MAX_CANVAS_SIDE, Raster, RasterOptions,
};
pub use raster_budget::MAX_COVERAGE_FACTOR;
pub use scene::{
    Affine, BitmapPaint, Cap, GradientKind, GradientPaint, Join, LayerKind, MAX_GRADIENT_STOPS,
    MAX_SCENE_MASKS, MAX_SCENE_PATHS, MAX_SCENE_PLACEMENTS, MAX_SCENE_RECORDS, MAX_SCENE_SEGMENTS,
    Origin, Paint, PathCommand, Scene, SceneItem, SceneMask, ScenePath, ScenePoint, SceneShape,
    ShapeLayer, Spread, StrokeStyle,
};
pub use scene_report::{SkipReason, Skipped};
pub use shape::{MAX_SHAPE_RECORDS, NewStyles, Shape, ShapeRecord, StyleChange};
pub use style::{FillStyle, Gradient, GradientStop, LineStyle};
pub use tag::tag_name;
pub use timeline::{Export, ExporterInfo, Sprite, Timeline};
pub use walker::MAX_SPRITE_DEPTH;
