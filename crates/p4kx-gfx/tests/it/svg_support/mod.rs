// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the SVG tests: movies of a 100 x 100 pixel stage, writing and parsing documents.

use std::collections::HashMap;

use p4kx_gfx::{Background, CanvasSize, GfxError, Movie, RasterOptions, RgbaImage, SvgReport};
use p4kx_testkit::{TestFill, TestMovie, TestShape, TestShapeRecord, TestSignature};
use roxmltree::{Document, Node};

pub(crate) const RED: [u8; 4] = [255, 0, 0, 255];
pub(crate) const BLUE: [u8; 4] = [0, 0, 255, 255];
pub(crate) const XLINK: &str = "http://www.w3.org/1999/xlink";

pub(crate) const fn stage() -> TestMovie {
    TestMovie::new(TestSignature::Gfx).stage(100, 100)
}

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

pub(crate) fn write_with(
    bytes: &[u8],
    options: &RasterOptions,
    title: Option<&str>,
    keep_bitmaps: bool,
) -> Result<(String, SvgReport), GfxError> {
    let movie = Movie::parse(bytes.to_vec())?;
    let assets = movie.assets();
    let scene = movie.first_frame(&assets);
    let mut bitmaps: HashMap<u16, RgbaImage> = HashMap::new();
    if keep_bitmaps {
        for id in scene.bitmap_ids() {
            if let Some(bitmap) = assets.bitmaps.iter().find(|bitmap| bitmap.id == id) {
                bitmaps.insert(id, assets.decode_bitmap(bitmap)?);
            }
        }
    }
    let mut out = Vec::new();
    let report = scene.write_svg(&bitmaps, options, title, &mut out)?;
    Ok((String::from_utf8(out).unwrap_or_default(), report))
}

pub(crate) const fn options(size: CanvasSize) -> RasterOptions {
    RasterOptions {
        size,
        background: Background::Transparent,
    }
}

pub(crate) fn svg(bytes: &[u8]) -> String {
    write_with(bytes, &options(CanvasSize::Stage), None, true).map_or_else(
        |error| unreachable!("cannot write: {error}"),
        |(text, _)| text,
    )
}

pub(crate) fn elements<'a, 'i>(document: &'a Document<'i>, name: &str) -> Vec<Node<'a, 'i>> {
    document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == name)
        .collect()
}

pub(crate) fn parse(text: &str) -> Document<'_> {
    Document::parse(text).unwrap_or_else(|error| unreachable!("not well-formed: {error}: {text}"))
}
