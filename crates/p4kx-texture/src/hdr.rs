use crate::bc6h::{self, HalfTexels};
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
        })
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
    /// signed or unsigned.
    #[must_use]
    pub const fn is_high_range(self) -> bool {
        half_decoder(self).is_some()
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
/// Every value is exactly the half float the format specifies; a reserved BC6H mode decodes to
/// zero.
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
    })
}
