// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Rendering the first frame of a UI movie in an Archive: external images and sub-images fill
//! shapes, and bitmaps that cannot be decoded are listed with their fills skipped.

use std::error::Error;
use std::path::Path;

use p4kx_extraction::gfx::{Background, CanvasSize, RasterOptions, SkipReason};
use p4kx_extraction::{Archive, ConversionError, Texture, UiMovie};
use p4kx_testkit::{
    ArchiveBuilder, SplitDds, TestEntry, TestFill, TestMovie, TestPlace, TestShape,
    TestShapeRecord, TestSignature, TestTextureFormat, external_image_body, sub_image_body,
};

type TestResult = Result<(), Box<dyn Error>>;

const MOVIE: &str = "Data\\UI\\Screens\\SWF\\Panel.gfx";
const TEXTURE: &str = "Data\\UI\\Screens\\Textures\\panel_bg.dds";

fn filled(id: u16, bitmap: u16, (width, height): (i32, i32)) -> TestShape {
    TestShape::new(id, 3, [0, width, 0, height])
        .fill(TestFill::ScaledBitmap {
            id: bitmap,
            scale: 20,
            at: (0, 0),
            repeating: false,
            smoothed: false,
        })
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

fn movie() -> Vec<u8> {
    let whole = filled(10, 2, (160, 80));
    let cut = filled(11, 3, (80, 40));
    let gone = filled(12, 5, (40, 40));
    TestMovie::new(TestSignature::Cfx)
        .stage(16, 8)
        .tag(
            1009,
            &external_image_body(2, 2, 19, (8, 4), ("bg", "../Textures/panel_bg.tif")),
        )
        .tag(1008, &sub_image_body(3, 2, [2, 1, 6, 3]))
        .tag(
            1009,
            &external_image_body(5, 2, 14, (8, 4), ("gone", "../Textures/gone.dds")),
        )
        .tag(whole.code(), &whole.body())
        .tag(cut.code(), &cut.body())
        .tag(gone.code(), &gone.body())
        .tag(26, &TestPlace::new(1, 10).body2())
        .tag(26, &TestPlace::new(2, 11).at(160, 0).body2())
        .tag(26, &TestPlace::new(3, 12).at(240, 80).body2())
        .tag(1, &[])
        .bytes()
}

fn rgba(pixels: &[u8], width: usize, (x, y): (usize, usize)) -> &[u8] {
    let at = (y * width + x) * 4;
    pixels.get(at..at + 4).unwrap_or_default()
}

/// Compares straight-alpha pixels, allowing for the precision a premultiplied canvas loses at
/// low opacity.
fn close(actual: &[u8], expected: &[u8]) -> bool {
    let alpha = expected.get(3).copied().unwrap_or(u8::MAX).max(1);
    let tolerance = 2 + 255 / alpha;
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(actual, expected)| actual.abs_diff(*expected) <= tolerance)
}

fn archive(dir: &Path) -> Result<Archive, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    let mut builder = ArchiveBuilder::new().entry(TestEntry::zstd(MOVIE, movie()));
    for entry in SplitDds::new(TestTextureFormat::LegacyBgra8, 8, 4).entries(TEXTURE) {
        builder = builder.entry(entry);
    }
    builder.write_to(&path)?;
    Ok(Archive::open_with_key(
        &path,
        Some(p4kx_testkit::test_key()),
    )?)
}

#[test]
fn external_images_and_sub_images_fill_shapes_and_missing_ones_are_listed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(dir.path())?;
    let texture = Texture::from_archive(&archive, TEXTURE)?.decode_mip(0)?;
    let full = texture.pixels();
    let movie = UiMovie::from_archive(&archive, MOVIE)?;
    let render = movie.render(&RasterOptions {
        size: CanvasSize::Stage,
        background: Background::Transparent,
    })?;
    let image = &render.image;
    assert_eq!((image.width(), image.height()), (16, 8), "stage");
    for (x, y) in [(0, 0), (7, 0), (3, 2), (7, 3)] {
        let actual = rgba(image.pixels(), 16, (x, y));
        let expected = rgba(full, 8, (x, y));
        assert!(
            close(actual, expected),
            "texture at ({x}, {y}): {actual:?} {expected:?}"
        );
    }
    for (x, y) in [(0, 0), (3, 1)] {
        let actual = rgba(image.pixels(), 16, (8 + x, y));
        let expected = rgba(full, 8, (2 + x, 1 + y));
        assert!(
            close(actual, expected),
            "cut at ({x}, {y}): {actual:?} {expected:?}"
        );
    }
    assert_eq!(
        rgba(image.pixels(), 16, (13, 5)),
        [0; 4],
        "nothing for the gone one"
    );
    assert_eq!(
        render.bitmap_failures.len(),
        1,
        "{:?}",
        render.bitmap_failures
    );
    let failure = &render.bitmap_failures[0];
    assert_eq!(failure.id, 5, "bitmap 5");
    assert!(
        matches!(&failure.error, ConversionError::ExternalImageMissing { .. }),
        "{}",
        failure.error
    );
    assert_eq!(
        render
            .skipped
            .iter()
            .map(|skipped| (skipped.id, skipped.reason))
            .collect::<Vec<_>>(),
        [(12, SkipReason::MissingBitmap { bitmap_id: 5 })],
        "its fill skipped"
    );
    assert!(!render.is_complete(), "incomplete");
    Ok(())
}
