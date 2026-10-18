use rayon::prelude::*;

use crate::bc::{self, Texels};
use crate::pixels::PixelFormat;
use crate::{TextureError, TextureFormat};
use crate::{bc6h, bc7, rgtc};

const BLOCK_SIDE: usize = 4;
const RGBA: usize = 4;

/// A decoded image: 8-bit RGBA pixels, row by row from the top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl RgbaImage {
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

    /// Returns the pixels, four bytes R, G, B, A each, row by row from the top.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Returns the pixels, giving up the image.
    #[must_use]
    pub fn into_pixels(self) -> Vec<u8> {
        self.pixels
    }
}

/// How one format is decoded: block by block for 8- or 16-byte blocks, or row by row.
#[derive(Debug, Clone, Copy)]
enum Decoder {
    Block8(fn([u8; 8]) -> Texels),
    Block16(fn([u8; 16]) -> Texels),
    Pixels(PixelFormat),
}

impl Decoder {
    const fn of(format: TextureFormat) -> Option<Self> {
        if let TextureFormat::Dxgi(dxgi) = format
            && let Some(decoder) = Self::block(dxgi.0)
        {
            return Some(decoder);
        }
        match PixelFormat::of(format) {
            Some(pixels) => Some(Self::Pixels(pixels)),
            None => None,
        }
    }

    const fn block(dxgi: u32) -> Option<Self> {
        Some(match dxgi {
            70..=72 => Self::Block8(bc::bc1),
            73..=75 => Self::Block16(bc::bc2),
            76..=78 => Self::Block16(bc::bc3),
            79 | 80 => Self::Block8(rgtc::bc4_unorm),
            81 => Self::Block8(rgtc::bc4_snorm),
            82 | 83 => Self::Block16(rgtc::bc5_unorm),
            84 => Self::Block16(rgtc::bc5_snorm),
            94 | 95 => Self::Block16(bc6h::bc6h_unsigned_rgba8),
            96 => Self::Block16(bc6h::bc6h_signed_rgba8),
            97..=99 => Self::Block16(bc7::bc7),
            _ => return None,
        })
    }
}

impl TextureFormat {
    /// Returns whether [`decode`] supports this format.
    #[must_use]
    pub const fn is_decodable(self) -> bool {
        Decoder::of(self).is_some()
    }
}

/// Decodes one image of `width` × `height` pixels in `format` to 8-bit RGBA, block rows in
/// parallel.
///
/// Single-channel formats become grey, alpha-only formats black with that alpha, two-channel
/// formats red and green; floating-point values are clamped to 0..=1. BC5 is a normal map: X in
/// red, Y in green and the reconstructed Z in blue. BC6H is tone-mapped to opaque sRGB: Reinhard
/// `v / (1 + v)` per channel, then the sRGB transfer function; [`decode_hdr`](crate::decode_hdr)
/// keeps its range. A reserved BC6H mode decodes to opaque black, a reserved BC7 mode to
/// transparent black.
///
/// # Errors
///
/// Returns [`TextureError::UnsupportedDecode`] when the format has no decoder,
/// [`TextureError::InvalidHeader`] when the size is zero or overflows, and
/// [`TextureError::MipLength`] when `data` is not exactly one image of that size.
pub fn decode(
    format: TextureFormat,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<RgbaImage, TextureError> {
    let decoder = Decoder::of(format).ok_or(TextureError::UnsupportedDecode(format))?;
    let size = ImageSize::of(format, width, height, data, RGBA)?;
    let mut pixels = vec![0; size.len];
    match decoder {
        Decoder::Block8(block) => decode_blocks(data, &size, &mut pixels, block),
        Decoder::Block16(block) => decode_blocks(data, &size, &mut pixels, block),
        Decoder::Pixels(format) => pixels
            .par_chunks_mut(size.out_row)
            .zip(data.par_chunks(size.row_len))
            .for_each(|(dst, src)| format.convert_row(src, dst)),
    }
    Ok(RgbaImage {
        width,
        height,
        pixels,
    })
}

/// The checked sizes of one image: its input rows, output rows and output length.
pub(crate) struct ImageSize {
    row_len: usize,
    out_row: usize,
    len: usize,
}

impl ImageSize {
    /// Checks that `data` is exactly one image of `width` × `height` pixels in `format`, decoded
    /// to `channels` values per pixel.
    pub(crate) fn of(
        format: TextureFormat,
        width: u32,
        height: u32,
        data: &[u8],
        channels: usize,
    ) -> Result<Self, TextureError> {
        let layout = format
            .layout()
            .ok_or(TextureError::UnsupportedDecode(format))?;
        if width == 0 || height == 0 {
            return Err(TextureError::InvalidHeader("width or height is zero"));
        }
        let row_len = u64::from(width.div_ceil(layout.block_width))
            .checked_mul(u64::from(layout.bytes_per_block))
            .ok_or_else(overflow)?;
        let expected = row_len
            .checked_mul(u64::from(height.div_ceil(layout.block_height)))
            .ok_or_else(overflow)?;
        if data.len() as u64 != expected {
            return Err(TextureError::MipLength {
                expected,
                actual: data.len() as u64,
            });
        }
        let out_row = to_usize(u64::from(width))?
            .checked_mul(channels)
            .ok_or_else(overflow)?;
        let len = out_row
            .checked_mul(to_usize(u64::from(height))?)
            .ok_or_else(overflow)?;
        Ok(Self {
            row_len: to_usize(row_len)?,
            out_row,
            len,
        })
    }

    /// Returns the number of output values.
    pub(crate) const fn len(&self) -> usize {
        self.len
    }
}

const fn overflow() -> TextureError {
    TextureError::InvalidHeader("the texture size overflows")
}

fn to_usize(value: u64) -> Result<usize, TextureError> {
    usize::try_from(value).map_err(|_| overflow())
}

/// Decodes the blocks of `data` into `pixels`, `C` values per pixel, block rows in parallel.
pub(crate) fn decode_blocks<const N: usize, const C: usize, T>(
    data: &[u8],
    size: &ImageSize,
    pixels: &mut [T],
    decode_block: impl Fn([u8; N]) -> [[T; C]; 16] + Sync,
) where
    T: Copy + Send + Sync,
{
    pixels
        .par_chunks_mut(size.out_row.saturating_mul(BLOCK_SIDE))
        .zip(data.par_chunks(size.row_len))
        .for_each_init(
            Vec::new,
            |decoded: &mut Vec<[[T; C]; 16]>, (rows, blocks)| {
                decoded.clear();
                let (blocks, _) = blocks.as_chunks::<N>();
                decoded.extend(blocks.iter().map(|block| decode_block(*block)));
                for (y, row) in rows.chunks_mut(size.out_row).enumerate() {
                    for (texels, block) in row.chunks_mut(BLOCK_SIDE * C).zip(decoded.iter()) {
                        let line = block.get(y * BLOCK_SIDE..).unwrap_or_default();
                        let (targets, _) = texels.as_chunks_mut::<C>();
                        for (target, texel) in targets.iter_mut().zip(line) {
                            *target = *texel;
                        }
                    }
                }
            },
        );
}
