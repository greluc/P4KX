// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::gfx::{BitWriter, rect, tag_bytes, zlib};
use crate::gfx_shape::{TestShapeRecord, write_records};

/// Returns the body of a 32-bit `DefineBitsLossless` (or with `alpha` `DefineBitsLossless2`)
/// tag holding straight-alpha RGBA `pixels`, premultiplied for the second.
#[must_use]
pub fn lossless_body(id: u16, width: u16, height: u16, pixels: &[[u8; 4]], alpha: bool) -> Vec<u8> {
    let mut raw = Vec::with_capacity(pixels.len() * 4);
    for &[red, green, blue, opacity] in pixels {
        if alpha {
            let premultiply = |value: u8| {
                u8::try_from(u16::from(value) * u16::from(opacity) / 255).unwrap_or(u8::MAX)
            };
            raw.extend_from_slice(&[
                opacity,
                premultiply(red),
                premultiply(green),
                premultiply(blue),
            ]);
        } else {
            raw.extend_from_slice(&[0xFF, red, green, blue]);
        }
    }
    let mut body = id.to_le_bytes().to_vec();
    body.push(5);
    body.extend_from_slice(&width.to_le_bytes());
    body.extend_from_slice(&height.to_le_bytes());
    body.extend_from_slice(&zlib(&raw));
    body
}

/// Returns the body of an 8-bit colour-mapped `DefineBitsLossless` tag: RGB `palette` and one
/// index per pixel, each row padded to four bytes.
#[must_use]
pub fn color_mapped_body(id: u16, width: u16, palette: &[[u8; 3]], rows: &[&[u8]]) -> Vec<u8> {
    let mut raw: Vec<u8> = palette.iter().flatten().copied().collect();
    for row in rows {
        raw.extend_from_slice(row);
        raw.resize(raw.len() + (4 - row.len() % 4) % 4, 0);
    }
    let mut body = id.to_le_bytes().to_vec();
    body.push(3);
    body.extend_from_slice(&width.to_le_bytes());
    body.extend_from_slice(&u16::try_from(rows.len()).unwrap_or(0).to_le_bytes());
    body.push(u8::try_from(palette.len().saturating_sub(1)).unwrap_or(u8::MAX));
    body.extend_from_slice(&zlib(&raw));
    body
}

/// Returns the body of a `DefineBitsJPEG2` tag (or with `alpha` a `DefineBitsJPEG3` tag with one
/// opacity byte per pixel) holding `image`, a JPEG, PNG or GIF file.
#[must_use]
pub fn jpeg_body(id: u16, image: &[u8], alpha: Option<&[u8]>) -> Vec<u8> {
    let mut body = id.to_le_bytes().to_vec();
    if let Some(alpha) = alpha {
        body.extend_from_slice(&u32::try_from(image.len()).unwrap_or(0).to_le_bytes());
        body.extend_from_slice(image);
        body.extend_from_slice(&zlib(alpha));
    } else {
        body.extend_from_slice(image);
    }
    body
}

/// Returns the body of a GFx `DefineExternalImage2` tag, or with `version` 1 of a
/// `DefineExternalImage` tag, referring to `file_name` in the GFx file `format`.
#[must_use]
pub fn external_image_body(
    id: u16,
    version: u8,
    format: u16,
    size: (u16, u16),
    names: (&str, &str),
) -> Vec<u8> {
    let mut body = id.to_le_bytes().to_vec();
    if version >= 2 {
        body.extend_from_slice(&0_u16.to_le_bytes());
    }
    body.extend_from_slice(&format.to_le_bytes());
    body.extend_from_slice(&size.0.to_le_bytes());
    body.extend_from_slice(&size.1.to_le_bytes());
    for name in <[&str; 2]>::from(names) {
        body.push(u8::try_from(name.len()).unwrap_or(u8::MAX));
        body.extend_from_slice(name.as_bytes());
    }
    if version >= 2 {
        body.push(0);
    }
    body
}

/// Returns the body of a GFx `DefineSubImage` tag cutting the rectangle x1, y1, x2, y2.
#[must_use]
pub fn sub_image_body(id: u16, image_id: u16, edges: [u16; 4]) -> Vec<u8> {
    let mut body = id.to_le_bytes().to_vec();
    body.extend_from_slice(&image_id.to_le_bytes());
    for edge in edges {
        body.extend_from_slice(&edge.to_le_bytes());
    }
    body
}

/// One glyph of [`font3_body`]: its code, advance and outline.
#[derive(Debug, Clone)]
pub struct TestGlyph {
    /// The UTF-16 code unit.
    pub code: u16,
    /// The advance.
    pub advance: i16,
    /// The outline; a [`TestShapeRecord::Move`] selects fill style 1.
    pub records: Vec<TestShapeRecord>,
}

/// Returns the body of a bold `DefineFont3` tag with layout, wide codes and one kerning pair
/// between the first two glyphs.
#[must_use]
pub fn font3_body(id: u16, name: &str, glyphs: &[TestGlyph]) -> Vec<u8> {
    let shapes: Vec<Vec<u8>> = glyphs
        .iter()
        .map(|glyph| {
            let mut bits = BitWriter::default();
            write_records(&mut bits, 1, 0, &glyph.records);
            bits.finish()
        })
        .collect();
    let mut body = id.to_le_bytes().to_vec();
    body.extend_from_slice(&[0x80 | 0x04 | 0x01, 1]);
    body.push(u8::try_from(name.len()).unwrap_or(u8::MAX));
    body.extend_from_slice(name.as_bytes());
    body.extend_from_slice(&u16::try_from(glyphs.len()).unwrap_or(0).to_le_bytes());
    let mut offset = (glyphs.len() + 1) * 2;
    for shape in &shapes {
        body.extend_from_slice(&u16::try_from(offset).unwrap_or(0).to_le_bytes());
        offset += shape.len();
    }
    body.extend_from_slice(&u16::try_from(offset).unwrap_or(0).to_le_bytes());
    for shape in &shapes {
        body.extend_from_slice(shape);
    }
    for glyph in glyphs {
        body.extend_from_slice(&glyph.code.to_le_bytes());
    }
    for value in [900_u16, 200, 0] {
        body.extend_from_slice(&value.to_le_bytes());
    }
    for glyph in glyphs {
        body.extend_from_slice(&glyph.advance.to_le_bytes());
    }
    let mut bounds = BitWriter::default();
    for _ in glyphs {
        rect(&mut bounds, [0, 1000, -900, 200]);
    }
    body.extend_from_slice(&bounds.finish());
    let pairs = glyphs.get(..2).map_or(0_u16, |_| 1);
    body.extend_from_slice(&pairs.to_le_bytes());
    if let [first, second, ..] = glyphs {
        body.extend_from_slice(&first.code.to_le_bytes());
        body.extend_from_slice(&second.code.to_le_bytes());
        body.extend_from_slice(&(-40_i16).to_le_bytes());
    }
    body
}

/// Returns the body of a `DefineFontName` tag.
#[must_use]
pub fn font_name_body(id: u16, full_name: &str) -> Vec<u8> {
    let mut body = id.to_le_bytes().to_vec();
    body.extend_from_slice(full_name.as_bytes());
    body.extend_from_slice(b"\0\0");
    body
}

/// Returns the body of a `DefineSprite` tag whose timeline holds `tags`, each a code and body,
/// followed by an `End` tag.
#[must_use]
pub fn sprite_body(id: u16, frame_count: u16, tags: &[(u16, Vec<u8>)]) -> Vec<u8> {
    let mut body = id.to_le_bytes().to_vec();
    body.extend_from_slice(&frame_count.to_le_bytes());
    for (code, tag) in tags {
        body.extend_from_slice(&tag_bytes(*code, tag));
    }
    body.extend_from_slice(&tag_bytes(0, &[]));
    body
}

/// Returns the body of a `PlaceObject2` tag placing the definition `id` at `depth`.
#[must_use]
pub fn place_object2_body(depth: u16, id: u16) -> Vec<u8> {
    let mut body = vec![0x02];
    body.extend_from_slice(&depth.to_le_bytes());
    body.extend_from_slice(&id.to_le_bytes());
    body
}

/// Returns the body of an `ExportAssets` tag.
#[must_use]
pub fn exports_body(exports: &[(u16, &str)]) -> Vec<u8> {
    let mut body = u16::try_from(exports.len())
        .unwrap_or(0)
        .to_le_bytes()
        .to_vec();
    for (id, name) in exports {
        body.extend_from_slice(&id.to_le_bytes());
        body.extend_from_slice(name.as_bytes());
        body.push(0);
    }
    body
}

/// Returns the body of a GFx `ExporterInfo` tag as the Star Citizen exporter writes it.
#[must_use]
pub fn exporter_info_body(swf_name: &str) -> Vec<u8> {
    let mut body = 0x036F_u16.to_le_bytes().to_vec();
    body.extend_from_slice(&0x10_u32.to_le_bytes());
    body.extend_from_slice(&19_u16.to_le_bytes());
    body.push(0);
    body.push(u8::try_from(swf_name.len()).unwrap_or(u8::MAX));
    body.extend_from_slice(swf_name.as_bytes());
    body.extend_from_slice(&0_u16.to_le_bytes());
    body
}
