// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! SVG output of bitmaps and masks: embedded PNG data, patterns, clips and nested masks.

use std::io::Cursor;

use crate::svg_support::{
    BLUE, RED, XLINK, elements, options, parse, rect, stage, svg, write_with,
};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use p4kx_gfx::{CanvasSize, SkipReason};
use p4kx_testkit::{TestFill, TestPlace, lossless_body, sprite_body};
use roxmltree::Document;

const GREEN: [u8; 4] = [0, 255, 0, 255];

fn quadrants(repeating: bool) -> Vec<u8> {
    let bitmap = lossless_body(7, 2, 2, &[RED, GREEN, BLUE, [255, 255, 255, 255]], false);
    let fill = TestFill::ScaledBitmap {
        id: 7,
        scale: 400,
        at: (0, 0),
        repeating,
        smoothed: false,
    };
    let square = rect(1, (1600, 800), fill);
    stage()
        .tag(20, &bitmap)
        .tag(square.code(), &square.body())
        .tag(26, &TestPlace::new(1, 1).body2())
        .tag(1, &[])
        .bytes()
}

fn embedded_pixels(document: &Document<'_>) -> (u32, u32, Vec<u8>) {
    let images = elements(document, "image");
    assert_eq!(images.len(), 1, "one bitmap embedded once");
    let href = images
        .first()
        .and_then(|image| image.attribute((XLINK, "href")))
        .unwrap_or_default();
    let data = href
        .strip_prefix("data:image/png;base64,")
        .unwrap_or_else(|| unreachable!("a PNG data URI: {href}"));
    let png = STANDARD
        .decode(data)
        .unwrap_or_else(|error| unreachable!("base64: {error}"));
    let mut reader = png::Decoder::new(Cursor::new(png))
        .read_info()
        .unwrap_or_else(|error| unreachable!("png: {error}"));
    let mut pixels = vec![0; reader.output_buffer_size().unwrap_or_default()];
    let info = reader
        .next_frame(&mut pixels)
        .unwrap_or_else(|error| unreachable!("png frame: {error}"));
    (info.width, info.height, pixels)
}

#[test]
fn bitmaps_are_embedded_as_png_and_repeat_as_patterns() {
    let text = svg(&quadrants(true));
    let document = parse(&text);
    let (width, height, pixels) = embedded_pixels(&document);
    assert_eq!((width, height), (2, 2));
    assert_eq!(
        pixels,
        [RED, GREEN, BLUE, [255, 255, 255, 255]].concat(),
        "the pixels survive"
    );
    let patterns = elements(&document, "pattern");
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].attribute("width"), Some("2"));
    assert_eq!(patterns[0].attribute("height"), Some("2"));
    assert_eq!(
        patterns[0].attribute("patternTransform"),
        Some("matrix(400 0 0 400 0 0)")
    );
    let uses = elements(&document, "use");
    assert_eq!(
        uses[0].attribute("image-rendering"),
        Some("optimizeSpeed"),
        "unsmoothed"
    );
    let path = &elements(&document, "path")[0];
    assert_eq!(path.attribute("fill"), Some("url(#p0_0)"));
    assert_eq!(patterns[0].attribute("id"), Some("p0_0"));
}

#[test]
fn a_bitmap_that_does_not_repeat_is_placed_once_inside_a_clip() {
    let text = svg(&quadrants(false));
    let document = parse(&text);
    embedded_pixels(&document);
    assert!(elements(&document, "pattern").is_empty(), "no tiling");
    assert_eq!(elements(&document, "clipPath").len(), 1);
    let clipped = document
        .descendants()
        .find(|node| node.attribute("clip-path").is_some())
        .unwrap_or_else(|| unreachable!("a clipped group"));
    let image = clipped
        .first_element_child()
        .map(|node| node.attribute("transform"));
    assert_eq!(image, Some(Some("matrix(400 0 0 400 0 0)")));
}

#[test]
fn a_colour_transformed_bitmap_is_embedded_transformed() {
    let bitmap = lossless_body(7, 1, 1, &[RED], false);
    let square = rect(
        1,
        (800, 800),
        TestFill::ScaledBitmap {
            id: 7,
            scale: 800,
            at: (0, 0),
            repeating: true,
            smoothed: true,
        },
    );
    let bytes = stage()
        .tag(20, &bitmap)
        .tag(square.code(), &square.body())
        .tag(
            26,
            &TestPlace::new(1, 1)
                .color([0.0, 1.0, 1.0, 1.0], [0, 255, 0, 0])
                .body2(),
        )
        .tag(1, &[])
        .bytes();
    let text = svg(&bytes);
    let document = parse(&text);
    let (_, _, pixels) = embedded_pixels(&document);
    assert_eq!(pixels, GREEN, "red turned green");
    assert_eq!(
        elements(&document, "use")[0].attribute("image-rendering"),
        None
    );
}

#[test]
fn a_bitmap_that_cannot_be_decoded_is_skipped_and_listed() {
    let bytes = quadrants(true);
    let (text, report) = write_with(&bytes, &options(CanvasSize::Stage), None, false)
        .unwrap_or_else(|error| unreachable!("{error}"));
    assert_eq!(report.skipped.len(), 1, "{:?}", report.skipped);
    assert_eq!(
        report.skipped[0].reason,
        SkipReason::MissingBitmap { bitmap_id: 7 }
    );
    let document = parse(&text);
    assert_eq!(elements(&document, "image").len(), 0);
    assert_eq!(elements(&document, "path").len(), 0);
    assert!(elements(&document, "use").len() <= 1);
}

fn masked() -> Vec<u8> {
    let mask = rect(1, (1000, 1000), TestFill::Solid(RED));
    let wide = rect(2, (2000, 2000), TestFill::Solid(BLUE));
    let strip = rect(3, (2000, 500), TestFill::Solid(RED));
    let small = rect(4, (400, 400), TestFill::Solid(GREEN));
    let inner = sprite_body(
        9,
        1,
        &[
            (26, TestPlace::new(1, 3).clip(2).body2()),
            (26, TestPlace::new(2, 2).body2()),
            (1, Vec::new()),
        ],
    );
    stage()
        .tag(mask.code(), &mask.body())
        .tag(wide.code(), &wide.body())
        .tag(strip.code(), &strip.body())
        .tag(small.code(), &small.body())
        .tag(39, &inner)
        .tag(26, &TestPlace::new(1, 1).clip(3).body2())
        .tag(26, &TestPlace::new(2, 2).body2())
        .tag(26, &TestPlace::new(3, 9).at(0, 1000).body2())
        .tag(26, &TestPlace::new(4, 4).at(1200, 1200).body2())
        .tag(1, &[])
        .bytes()
}

#[test]
fn masks_are_written_nested_and_the_items_they_clip_sit_inside_them() {
    let text = svg(&masked());
    let document = parse(&text);
    let masks = elements(&document, "mask");
    assert_eq!(masks.len(), 2);
    assert_eq!(masks[0].attribute("id"), Some("m0"));
    assert_eq!(masks[1].attribute("id"), Some("m1"));
    assert!(
        masks[1]
            .children()
            .any(|node| node.attribute("mask") == Some("url(#m0)")),
        "the second is clipped by the first"
    );
    for path in masks[0]
        .descendants()
        .filter(|node| node.has_tag_name("path"))
    {
        assert_eq!(path.attribute("fill"), Some("#fff"));
    }
    let clipped: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name("g") && node.attribute("mask").is_some())
        .filter(|node| node.parent().is_some_and(|parent| parent.has_tag_name("g")))
        .collect();
    assert!(
        clipped
            .iter()
            .any(|node| node.attribute("mask") == Some("url(#m1)")),
        "items under the inner mask"
    );
}
