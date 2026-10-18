//! Reads the GFx and SWF user-interface movies of Star Citizen Archives: the file and movie
//! header with zlib or LZMA compression, the tag stream with unknown tags kept, and the shapes,
//! morph shapes, bitmaps (embedded, GFx external and sub-images), fonts and sprites a movie
//! defines. Lossless and embedded bitmaps decode to RGBA and write as PNG; shapes serialise to
//! JSON in the units the file stores.
//!
//! Every size is checked against the data left and the limits [`MAX_MOVIE_LEN`],
//! [`MAX_BITMAP_PIXELS`], [`MAX_SHAPE_RECORDS`] and [`MAX_SPRITE_DEPTH`]; corrupt input yields a
//! [`GfxError`], never a panic.

mod assets;
mod bitmap;
mod error;
mod font;
mod font_info;
mod geometry;
mod header;
mod image;
mod jpeg;
mod json;
mod line;
mod lossless;
mod morph;
mod movie;
mod reader;
mod shape;
mod style;
mod tag;
mod timeline;
mod walker;

pub use assets::{Assets, TagCount, TagFailure, UnknownTag};
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
pub use shape::{MAX_SHAPE_RECORDS, NewStyles, Shape, ShapeRecord, StyleChange};
pub use style::{FillStyle, Gradient, GradientStop, LineStyle};
pub use tag::tag_name;
pub use timeline::{Export, ExporterInfo, Sprite, Timeline};
pub use walker::MAX_SPRITE_DEPTH;
