// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::bc6h::{self, HalfTexels};
use crate::floats::FloatFormat;
use crate::image::{ImageSize, decode_blocks};
use crate::pixels::half_to_f32;
use crate::{TextureError, TextureFormat};

const RGB: usize = 3;

/// A decoded high-range image: linear floating-point RGB pixels, row by row from the top.
#[derive(Debug, Clone, PartialEq)]
pub struct HdrImage {
    width: u32,
    height: u32,
    pixels: Vec<f32>,
    alpha: Option<Vec<f32>>,
}

impl HdrImage {
    /// Creates an image of `width` × `height` pixels from three values R, G, B per pixel, row by
    /// row from the top.
    ///
    /// # Errors
    ///
    /// Returns [`TextureError::MipLength`] when `pixels` does not hold exactly that many pixels,
    /// and [`TextureError::InvalidHeader`] when the size overflows.
    pub fn new(width: u32, height: u32, pixels: Vec<f32>) -> Result<Self, TextureError> {
        let expected = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|count| count.checked_mul(RGB as u64))
            .ok_or(TextureError::InvalidHeader("the texture size overflows"))?;
        if pixels.len() as u64 != expected {
            return Err(TextureError::MipLength {
                expected,
                actual: pixels.len() as u64,
            });
        }
        Ok(Self {
            width,
            height,
            pixels,
            alpha: None,
        })
    }

    /// Returns the image with the alpha channel `alpha`, one value per pixel, row by row from
    /// the top.
    ///
    /// # Errors
    ///
    /// Returns [`TextureError::MipLength`] when `alpha` does not hold one value per pixel.
    pub fn with_alpha(mut self, alpha: Vec<f32>) -> Result<Self, TextureError> {
        let expected = self.pixels.len() as u64 / RGB as u64;
        if alpha.len() as u64 != expected {
            return Err(TextureError::MipLength {
                expected,
                actual: alpha.len() as u64,
            });
        }
        self.alpha = Some(alpha);
        Ok(self)
    }

    /// Returns the alpha channel, one value per pixel, or `None` when the image has none.
    #[must_use]
    pub fn alpha(&self) -> Option<&[f32]> {
        self.alpha.as_deref()
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

    /// Returns the pixels, three values R, G, B each, row by row from the top.
    #[must_use]
    pub fn pixels(&self) -> &[f32] {
        &self.pixels
    }
}

impl TextureFormat {
    /// Returns whether the format holds values beyond 0..=1 that [`decode_hdr`] keeps: BC6H,
    /// signed or unsigned, and the floating-point formats.
    #[must_use]
    pub const fn is_high_range(self) -> bool {
        half_decoder(self).is_some() || FloatFormat::of(self).is_some()
    }
}

const fn half_decoder(format: TextureFormat) -> Option<fn([u8; 16]) -> HalfTexels> {
    match bc6h::signedness(format) {
        Some(true) => Some(bc6h::bc6h_signed),
        Some(false) => Some(bc6h::bc6h_unsigned),
        None => None,
    }
}

/// Decodes one image of `width` × `height` pixels in a high-range `format` to linear
/// floating-point RGB without clipping its range, block rows in parallel.
///
/// Every value is exactly the float the format specifies, a half float widened for BC6H and the
/// 16-bit formats; a reserved BC6H mode decodes to zero. Channels a format lacks are zero; the
/// four-channel formats keep their alpha.
///
/// # Errors
///
/// Returns [`TextureError::NotHighRange`] when the format is not high-range,
/// [`TextureError::InvalidHeader`] when the size is zero or overflows, and
/// [`TextureError::MipLength`] when `data` is not exactly one image of that size.
pub fn decode_hdr(
    format: TextureFormat,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<HdrImage, TextureError> {
    if let Some(floats) = FloatFormat::of(format) {
        let size = ImageSize::of(format, width, height, data, RGB)?;
        let mut pixels = vec![0.0; size.len()];
        size.for_each_row(data, &mut pixels, |src, dst| floats.rgb_row(src, dst));
        let image = HdrImage {
            width,
            height,
            pixels,
            alpha: None,
        };
        if !floats.has_alpha() {
            return Ok(image);
        }
        let alpha_size = ImageSize::of(format, width, height, data, 1)?;
        let mut alpha = vec![0.0; alpha_size.len()];
        alpha_size.for_each_row(data, &mut alpha, |src, dst| floats.alpha_row(src, dst));
        return image.with_alpha(alpha);
    }
    let decode_block = half_decoder(format).ok_or(TextureError::NotHighRange(format))?;
    let size = ImageSize::of(format, width, height, data, RGB)?;
    let mut pixels = vec![0.0; size.len()];
    decode_blocks(data, &size, &mut pixels, |block| {
        decode_block(block).map(|texel| texel.map(half_to_f32))
    });
    Ok(HdrImage {
        width,
        height,
        pixels,
        alpha: None,
    })
}
