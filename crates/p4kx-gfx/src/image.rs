// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{Read, Write};

use flate2::read::ZlibDecoder;
use png::{BitDepth, ColorType, Compression, Encoder, EncodingError};

use crate::GfxError;

/// The most pixels a decoded bitmap may have, above the largest UI texture of a real Archive with
/// 22.3 million pixels.
pub const MAX_BITMAP_PIXELS: u64 = 1 << 25;

/// A decoded bitmap: 8-bit RGBA pixels with straight alpha, row by row from the top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl RgbaImage {
    /// Wraps `pixels`, which must hold `width * height * 4` bytes.
    pub(crate) const fn new(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Wraps decoded RGBA `pixels`, e.g. of an external image.
    ///
    /// # Errors
    ///
    /// Returns [`GfxError::BitmapTooLarge`] above [`MAX_BITMAP_PIXELS`] and
    /// [`GfxError::BitmapData`] when `pixels` does not hold `width * height * 4` bytes.
    pub fn from_pixels(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, GfxError> {
        let count = checked_pixels(u64::from(width), u64::from(height))?;
        if pixels.len() != count * 4 {
            return Err(GfxError::BitmapData {
                message: format!("{} bytes for {width} x {height} pixels", pixels.len()),
            });
        }
        Ok(Self::new(width, height, pixels))
    }

    /// Returns the rectangle from (`x1`, `y1`) up to but excluding (`x2`, `y2`), cut to the
    /// image; `None` when nothing of it lies inside.
    #[must_use]
    pub fn crop(&self, (x1, y1): (u32, u32), (x2, y2): (u32, u32)) -> Option<Self> {
        let x2 = x2.min(self.width);
        let y2 = y2.min(self.height);
        if x1 >= x2 || y1 >= y2 {
            return None;
        }
        let row_len = self.width as usize * 4;
        let mut pixels = Vec::with_capacity((x2 - x1) as usize * (y2 - y1) as usize * 4);
        for row in self
            .pixels
            .chunks_exact(row_len)
            .skip(y1 as usize)
            .take((y2 - y1) as usize)
        {
            pixels.extend_from_slice(row.get(x1 as usize * 4..x2 as usize * 4)?);
        }
        Some(Self::new(x2 - x1, y2 - y1, pixels))
    }

    /// Returns the width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the pixels, four bytes R, G, B, A each.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Returns the pixels for changing them in place.
    pub(crate) fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// Writes the image to `out` as an 8-bit RGBA PNG.
    ///
    /// # Errors
    ///
    /// Returns [`GfxError::Write`] when `out` fails and [`GfxError::EncodePng`] when the image
    /// cannot be encoded.
    pub fn write_png<W: Write>(&self, out: W) -> Result<(), GfxError> {
        let mut encoder = Encoder::new(out, self.width, self.height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_compression(Compression::Fast);
        let mut writer = encoder.write_header().map_err(png_error)?;
        writer.write_image_data(&self.pixels).map_err(png_error)?;
        writer.finish().map_err(png_error)
    }
}

fn png_error(error: EncodingError) -> GfxError {
    match error {
        EncodingError::IoError(source) => GfxError::Write(source),
        other => GfxError::EncodePng(other.to_string()),
    }
}

/// Returns the pixel count of a `width` x `height` bitmap, or an error above
/// [`MAX_BITMAP_PIXELS`].
pub(crate) fn checked_pixels(width: u64, height: u64) -> Result<usize, GfxError> {
    width
        .checked_mul(height)
        .filter(|&pixels| pixels <= MAX_BITMAP_PIXELS)
        .and_then(|pixels| usize::try_from(pixels).ok())
        .ok_or(GfxError::BitmapTooLarge {
            width,
            height,
            limit: MAX_BITMAP_PIXELS,
        })
}

const MAX_DEFLATE_RATIO: usize = 1032;

/// Inflates the zlib stream `data`, which must hold at least `expected` bytes; more are ignored.
pub(crate) fn inflate_exact(data: &[u8], expected: usize) -> Result<Vec<u8>, GfxError> {
    let mut bytes = Vec::with_capacity(expected.min(data.len().saturating_mul(MAX_DEFLATE_RATIO)));
    ZlibDecoder::new(data)
        .take(expected as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| GfxError::BitmapData {
            message: error.to_string(),
        })?;
    if bytes.len() < expected {
        return Err(GfxError::BitmapData {
            message: format!("{} of {expected} bytes", bytes.len()),
        });
    }
    Ok(bytes)
}
