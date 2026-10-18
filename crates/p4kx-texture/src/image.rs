use rayon::prelude::*;

use crate::bc::{self, Texels};
use crate::pixels::PixelFormat;
use crate::{TextureError, TextureFormat};
use crate::{bc7, rgtc};

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
/// red, Y in green and the reconstructed Z in blue. A reserved BC7 mode decodes to transparent
/// black.
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
    let (row_len, width_px) = (to_usize(row_len)?, to_usize(u64::from(width))?);
    let out_row = width_px.checked_mul(RGBA).ok_or_else(overflow)?;
    let len = out_row
        .checked_mul(to_usize(u64::from(height))?)
        .ok_or_else(overflow)?;
    let mut pixels = vec![0; len];
    match decoder {
        Decoder::Block8(block) => decode_blocks(data, row_len, out_row, &mut pixels, block),
        Decoder::Block16(block) => decode_blocks(data, row_len, out_row, &mut pixels, block),
        Decoder::Pixels(format) => pixels
            .par_chunks_mut(out_row)
            .zip(data.par_chunks(row_len))
            .for_each(|(dst, src)| format.convert_row(src, dst)),
    }
    Ok(RgbaImage {
        width,
        height,
        pixels,
    })
}

const fn overflow() -> TextureError {
    TextureError::InvalidHeader("the texture size overflows")
}

fn to_usize(value: u64) -> Result<usize, TextureError> {
    usize::try_from(value).map_err(|_| overflow())
}

fn decode_blocks<const N: usize>(
    data: &[u8],
    row_len: usize,
    out_row: usize,
    pixels: &mut [u8],
    decode_block: impl Fn([u8; N]) -> Texels + Sync,
) {
    pixels
        .par_chunks_mut(out_row.saturating_mul(BLOCK_SIDE))
        .zip(data.par_chunks(row_len))
        .for_each_init(Vec::new, |decoded: &mut Vec<Texels>, (rows, blocks)| {
            decoded.clear();
            let (blocks, _) = blocks.as_chunks::<N>();
            decoded.extend(blocks.iter().map(|block| decode_block(*block)));
            for (y, row) in rows.chunks_mut(out_row).enumerate() {
                for (texels, block) in row.chunks_mut(BLOCK_SIDE * RGBA).zip(decoded.iter()) {
                    let line = block.get(y * BLOCK_SIDE..).unwrap_or_default();
                    let (targets, _) = texels.as_chunks_mut::<RGBA>();
                    for (target, texel) in targets.iter_mut().zip(line) {
                        *target = *texel;
                    }
                }
            }
        });
}
