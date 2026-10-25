// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::TextureFormat;
use crate::pixels::{float_unorm, half_to_f32};
use crate::tone_map::tone_map;

/// An uncompressed floating-point pixel format. Its values are not limited to 0..=1, so it is a
/// high-range format: [`decode_hdr`](crate::decode_hdr) keeps them and the 8-bit decode
/// tone-maps them like BC6H.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FloatFormat {
    R16,
    Rg16,
    Rgba16,
    R32,
    Rg32,
    Rgb32,
    Rgba32,
    R11G11B10,
    Rgb9E5,
}

impl FloatFormat {
    /// Returns the floating-point format of `format`, or `None` for any other.
    pub(crate) const fn of(format: TextureFormat) -> Option<Self> {
        let TextureFormat::Dxgi(dxgi) = format else {
            return None;
        };
        Some(match dxgi.0 {
            54 => Self::R16,
            34 => Self::Rg16,
            10 => Self::Rgba16,
            41 => Self::R32,
            16 => Self::Rg32,
            6 => Self::Rgb32,
            2 => Self::Rgba32,
            26 => Self::R11G11B10,
            67 => Self::Rgb9E5,
            _ => return None,
        })
    }

    /// Calls `put` with the index and the red, green, blue and alpha values of every pixel of the
    /// row `src`. Channels the format lacks are zero, alpha 1 when the format has none.
    pub(crate) fn each(self, src: &[u8], mut put: impl FnMut(usize, [f32; 4])) {
        let half = |bytes: [u8; 2]| half_to_f32(u16::from_le_bytes(bytes));
        match self {
            Self::R16 => walk(src, &mut put, |[l, h]| {
                let r = half([l, h]);
                [r, r, r, 1.0]
            }),
            Self::Rg16 => walk(src, &mut put, |[r0, r1, g0, g1]| {
                [half([r0, r1]), half([g0, g1]), 0.0, 1.0]
            }),
            Self::Rgba16 => walk(src, &mut put, |[r0, r1, g0, g1, b0, b1, a0, a1]| {
                [
                    half([r0, r1]),
                    half([g0, g1]),
                    half([b0, b1]),
                    half([a0, a1]),
                ]
            }),
            Self::R32 => walk(src, &mut put, |bytes: [u8; 4]| {
                let r = f32::from_le_bytes(bytes);
                [r, r, r, 1.0]
            }),
            Self::Rg32 => walk(src, &mut put, |bytes: [u8; 8]| {
                let [r, g] = singles::<2>(&bytes);
                [r, g, 0.0, 1.0]
            }),
            Self::Rgb32 => walk(src, &mut put, |bytes: [u8; 12]| {
                let [r, g, b] = singles::<3>(&bytes);
                [r, g, b, 1.0]
            }),
            Self::Rgba32 => walk(src, &mut put, |bytes: [u8; 16]| singles::<4>(&bytes)),
            Self::R11G11B10 => walk(src, &mut put, |bytes: [u8; 4]| {
                let value = u32::from_le_bytes(bytes);
                [
                    small_float(value & 0x7FF, 6),
                    small_float((value >> 11) & 0x7FF, 6),
                    small_float(value >> 22, 5),
                    1.0,
                ]
            }),
            Self::Rgb9E5 => walk(src, &mut put, |bytes: [u8; 4]| {
                let value = u32::from_le_bytes(bytes);
                let scale = shared_exponent(value >> 27);
                let channel = |shift: u32| f32::from(mantissa9((value >> shift) & 0x1FF)) * scale;
                [channel(0), channel(9), channel(18), 1.0]
            }),
        }
    }

    /// Converts the row `src` into the 8-bit RGBA row `dst`: red, green and blue tone-mapped like
    /// BC6H, alpha clamped to 0..=1.
    pub(crate) fn rgba8_row(self, src: &[u8], dst: &mut [u8]) {
        let (targets, _) = dst.as_chunks_mut::<4>();
        self.each(src, |index, [r, g, b, a]| {
            if let Some(target) = targets.get_mut(index) {
                *target = [tone_map(r), tone_map(g), tone_map(b), float_unorm(a)];
            }
        });
    }

    /// Returns whether the format stores an alpha channel.
    pub(crate) const fn has_alpha(self) -> bool {
        matches!(self, Self::Rgba16 | Self::Rgba32)
    }

    /// Converts the row `src` into the alpha row `dst`, one value per pixel.
    pub(crate) fn alpha_row(self, src: &[u8], dst: &mut [f32]) {
        self.each(src, |index, [_, _, _, a]| {
            if let Some(target) = dst.get_mut(index) {
                *target = a;
            }
        });
    }

    /// Converts the row `src` into the floating-point RGB row `dst`.
    pub(crate) fn rgb_row(self, src: &[u8], dst: &mut [f32]) {
        let (targets, _) = dst.as_chunks_mut::<3>();
        self.each(src, |index, [r, g, b, _]| {
            if let Some(target) = targets.get_mut(index) {
                *target = [r, g, b];
            }
        });
    }
}

fn walk<const N: usize>(
    src: &[u8],
    put: &mut impl FnMut(usize, [f32; 4]),
    read: impl Fn([u8; N]) -> [f32; 4],
) {
    let (pixels, _) = src.as_chunks::<N>();
    for (index, pixel) in pixels.iter().enumerate() {
        put(index, read(*pixel));
    }
}

/// Reads `C` little-endian floats from the start of `bytes`.
fn singles<const C: usize>(bytes: &[u8]) -> [f32; C] {
    let (chunks, _) = bytes.as_chunks::<4>();
    let mut values = [0.0; C];
    for (value, chunk) in values.iter_mut().zip(chunks) {
        *value = f32::from_le_bytes(*chunk);
    }
    values
}

/// Decodes an unsigned float of 5 exponent bits and `mantissa_bits` mantissa bits.
fn small_float(bits: u32, mantissa_bits: u32) -> f32 {
    let mantissa = bits & ((1 << mantissa_bits) - 1);
    let exponent = (bits >> mantissa_bits) & 0x1F;
    let fraction = f32::from(u16::try_from(mantissa).unwrap_or_default());
    let steps = f32::from(1_u16 << mantissa_bits);
    match (exponent, mantissa) {
        (0, _) => fraction / steps * 2.0_f32.powi(-14),
        (0x1F, 0) => f32::INFINITY,
        (0x1F, _) => f32::NAN,
        _ => (1.0 + fraction / steps) * 2.0_f32.powi(i32::try_from(exponent).unwrap_or(0) - 15),
    }
}

fn mantissa9(bits: u32) -> u16 {
    u16::try_from(bits).unwrap_or_default()
}

/// The factor `2^(exponent - 15) / 512` the three mantissas of R9G9B9E5 share.
fn shared_exponent(exponent: u32) -> f32 {
    2.0_f32.powi(i32::try_from(exponent).unwrap_or(0) - 15 - 9)
}
