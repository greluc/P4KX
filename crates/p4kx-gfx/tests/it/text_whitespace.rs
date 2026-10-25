// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Whitespace of a text field draws no boxes, whatever its font holds.

use std::collections::HashMap;

use crate::support::{CLEAR, pixel, stage};
use p4kx_gfx::{Background, CanvasSize, Movie, Raster, RasterOptions};
use p4kx_testkit::{TestEditText, TestGlyph, TestPlace, font3_body};

const FIELD: [i32; 4] = [0, 2000, 800, 1400];
const FONT: u16 = 3;

fn render(glyphs: &[TestGlyph]) -> Raster {
    let spaces = TestEditText::new(7, FIELD)
        .text("   ")
        .font(FONT, 400)
        .color([0, 255, 0, 255]);
    let bytes = stage()
        .tag(75, &font3_body(FONT, "Orbitron", glyphs))
        .tag(37, &spaces.body())
        .tag(26, &TestPlace::new(1, 7).body2())
        .tag(1, &[])
        .bytes();
    let movie = Movie::parse(bytes).unwrap_or_else(|error| unreachable!("{error}"));
    let assets = movie.assets();
    assert!(assets.failures.is_empty(), "{:?}", assets.failures);
    let options = RasterOptions {
        size: CanvasSize::Stage,
        background: Background::Transparent,
    };
    movie
        .first_frame(&assets)
        .rasterize(&HashMap::new(), &options)
        .unwrap_or_else(|error| unreachable!("{error}"))
}

fn drawn(raster: &Raster) -> usize {
    (0..100)
        .flat_map(|y| (0..100).map(move |x| (x, y)))
        .filter(|&(x, y)| pixel(&raster.image, x, y) != CLEAR)
        .count()
}

#[test]
fn a_font_that_holds_only_an_outline_free_space_glyph_draws_no_boxes_for_spaces() {
    let space = TestGlyph {
        code: u16::from(b' '),
        advance: 6_000,
        records: Vec::new(),
    };
    let raster = render(&[space]);
    assert_eq!(drawn(&raster), 0, "no box for a space");
    assert!(raster.skipped.is_empty(), "{:?}", raster.skipped);
}

#[test]
fn a_font_without_a_space_glyph_draws_no_boxes_for_spaces() {
    let letter = TestGlyph {
        code: u16::from(b'A'),
        advance: 12_000,
        records: Vec::new(),
    };
    let raster = render(&[letter]);
    assert_eq!(drawn(&raster), 0, "no box for a missing space glyph");
}
