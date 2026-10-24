// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the render tests: movies of a 100 x 100 pixel stage, rendering and pixel checks.

use std::collections::HashMap;

use p4kx_gfx::{Background, CanvasSize, Movie, Raster, RasterOptions, RgbaImage, Scene};
use p4kx_testkit::{TestFill, TestMovie, TestShape, TestShapeRecord, TestSignature};

/// Opaque red.
pub(crate) const RED: [u8; 4] = [255, 0, 0, 255];
/// Opaque blue.
pub(crate) const BLUE: [u8; 4] = [0, 0, 255, 255];
/// Fully transparent.
pub(crate) const CLEAR: [u8; 4] = [0, 0, 0, 0];

/// Starts a movie with a 100 x 100 pixel stage.
pub(crate) const fn stage() -> TestMovie {
    TestMovie::new(TestSignature::Gfx).stage(100, 100)
}

/// A `DefineShape3` rectangle from (0, 0) to `width` x `height` twips filled with `fill`.
pub(crate) fn rect(id: u16, (width, height): (i32, i32), fill: TestFill) -> TestShape {
    TestShape::new(id, 3, [0, width, 0, height])
        .fill(fill)
        .record(TestShapeRecord::Move {
            to: (0, 0),
            fill0: 0,
            fill1: 1,
            line: 0,
        })
        .record(TestShapeRecord::Line(width, 0))
        .record(TestShapeRecord::Line(0, height))
        .record(TestShapeRecord::Line(-width, 0))
        .record(TestShapeRecord::Line(0, -height))
}

/// Parses a movie and composes its first frame.
pub(crate) fn scene(bytes: &[u8]) -> (Movie, Scene) {
    let movie =
        Movie::parse(bytes.to_vec()).unwrap_or_else(|error| unreachable!("invalid movie: {error}"));
    let assets = movie.assets();
    assert!(assets.failures.is_empty(), "{:?}", assets.failures);
    let scene = movie.first_frame(&assets);
    (movie, scene)
}

/// Renders the first frame of a movie at `size` without background, decoding its embedded
/// bitmaps.
pub(crate) fn render(bytes: &[u8], size: CanvasSize) -> Raster {
    let (movie, scene) = scene(bytes);
    let assets = movie.assets();
    let mut bitmaps: HashMap<u16, RgbaImage> = HashMap::new();
    for id in scene.bitmap_ids() {
        if let Some(bitmap) = assets.bitmaps.iter().find(|bitmap| bitmap.id == id)
            && let Ok(image) = assets.decode_bitmap(bitmap)
        {
            bitmaps.insert(id, image);
        }
    }
    let options = RasterOptions {
        size,
        background: Background::Transparent,
    };
    scene
        .rasterize(&bitmaps, &options)
        .unwrap_or_else(|error| unreachable!("cannot render: {error}"))
}

/// Returns the pixel at (x, y).
pub(crate) fn pixel(image: &RgbaImage, x: u32, y: u32) -> [u8; 4] {
    let at = (y * image.width() + x) as usize * 4;
    let mut pixel = [0; 4];
    if let Some(bytes) = image.pixels().get(at..at + 4) {
        pixel.copy_from_slice(bytes);
    }
    pixel
}

/// Asserts that the pixel at (x, y) is `expected`, each channel within `tolerance`.
pub(crate) fn assert_pixel(
    image: &RgbaImage,
    (x, y): (u32, u32),
    expected: [u8; 4],
    tolerance: u8,
) {
    let actual = pixel(image, x, y);
    let close = actual
        .iter()
        .zip(expected)
        .all(|(&actual, expected)| actual.abs_diff(expected) <= tolerance);
    assert!(
        close,
        "pixel ({x}, {y}) is {actual:?}, expected {expected:?}"
    );
}
