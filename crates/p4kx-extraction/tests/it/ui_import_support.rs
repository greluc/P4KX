// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Synthetic fonts, clients and Archives for the tests of imported UI fonts.

use std::error::Error;
use std::path::Path;

use p4kx_extraction::Archive;
use p4kx_extraction::gfx::{Background, CanvasSize, RasterOptions};
use p4kx_testkit::{
    ArchiveBuilder, TestEditText, TestEntry, TestGlyph, TestMovie, TestPlace, TestShapeRecord,
    TestSignature, exports_body, font3_body, import_assets_body,
};

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

pub(crate) const GREEN: [u8; 4] = [0, 255, 0, 255];
pub(crate) const FIELD: [i32; 4] = [0, 2000, 800, 1400];
pub(crate) const CLIENT: &str = "Data\\UI\\Screens\\SWF\\Panel.swf";
pub(crate) const FONTS: &str = "Data\\UI\\fonts\\Shared\\fonts_en.gfx";

pub(crate) fn glyph(width: i16) -> TestGlyph {
    TestGlyph {
        code: u16::from(b'A'),
        advance: 12_000,
        records: vec![
            TestShapeRecord::Move {
                to: (0, 0),
                fill0: 0,
                fill1: 1,
                line: 0,
            },
            TestShapeRecord::Line(i32::from(width), 0),
            TestShapeRecord::Line(0, -14_000),
            TestShapeRecord::Line(-i32::from(width), 0),
            TestShapeRecord::Line(0, 14_000),
        ],
    }
}

pub(crate) fn square() -> TestGlyph {
    glyph(10_000)
}

pub(crate) const fn stage() -> TestMovie {
    TestMovie::new(TestSignature::Gfx).stage(100, 100)
}

pub(crate) fn font_movie_with(glyph: TestGlyph, export: &str) -> Vec<u8> {
    stage()
        .tag(75, &font3_body(4, "Orbitron", &[glyph]))
        .tag(56, &exports_body(&[(4, export)]))
        .bytes()
}

pub(crate) fn font_movie(export: &str) -> Vec<u8> {
    font_movie_with(square(), export)
}

pub(crate) fn client(url: &str, name: &str) -> Vec<u8> {
    let text = TestEditText::new(7, FIELD)
        .text("A")
        .font(9, 400)
        .color(GREEN);
    stage()
        .tag(71, &import_assets_body(url, 2, &[(9, name)]))
        .tag(37, &text.body())
        .tag(26, &TestPlace::new(1, 7).body2())
        .tag(1, &[])
        .bytes()
}

/// One text field per entry of `fields`, each in the font `.0` and moved down by `.1` twips.
pub(crate) fn client_of(
    base: TestMovie,
    imports: &[(&str, u16, &str)],
    fields: &[(u16, i32)],
) -> Vec<u8> {
    let mut movie = base;
    for &(url, id, name) in imports {
        movie = movie.tag(71, &import_assets_body(url, 2, &[(id, name)]));
    }
    for (depth, &(font, down)) in (1_u16..).zip(fields) {
        let id = 1_000 + depth;
        let text = TestEditText::new(id, FIELD)
            .text("A")
            .font(font, 400)
            .color(GREEN);
        movie = movie
            .tag(37, &text.body())
            .tag(26, &TestPlace::new(depth, id).at(0, down).body2());
    }
    movie.tag(1, &[]).bytes()
}

pub(crate) fn archive(dir: &Path, entries: Vec<TestEntry>) -> Result<Archive, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    let mut builder = ArchiveBuilder::new();
    for entry in entries {
        builder = builder.entry(entry);
    }
    builder.write_to(&path)?;
    Ok(Archive::open_with_key(
        &path,
        Some(p4kx_testkit::test_key()),
    )?)
}

pub(crate) const fn options() -> RasterOptions {
    RasterOptions {
        size: CanvasSize::Stage,
        background: Background::Transparent,
    }
}

pub(crate) fn pixel(pixels: &[u8], (x, y): (usize, usize)) -> &[u8] {
    let at = (y * 100 + x) * 4;
    pixels.get(at..at + 4).unwrap_or_default()
}

pub(crate) fn green_pixels(pixels: &[u8], rows: std::ops::Range<usize>) -> usize {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .skip(rows.start * 100)
        .take(rows.len() * 100)
        .filter(|pixel| **pixel == GREEN)
        .count()
}
