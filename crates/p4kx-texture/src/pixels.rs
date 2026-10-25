// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::TextureFormat;
use crate::rgtc::snorm_to_unorm;

/// An uncompressed pixel format the decoder converts to 8-bit RGBA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PixelFormat {
    Rgba8,
    Bgra8,
    Bgrx8,
    Bgr8,
    L8A8,
    R8,
    A8,
    Rg8,
    R16,
    R32,
    Rg16,
    Rgba16,
    Rgb10A2,
    B5G6R5,
    B5G5R5A1,
    B4G4R4A4,
    R8Snorm,
    Rg8Snorm,
    Rgba8Snorm,
}

impl PixelFormat {
    /// Returns the decoder of `format`, or `None` when it is not an uncompressed format this
    /// version decodes.
    pub(crate) const fn of(format: TextureFormat) -> Option<Self> {
        let code = match format {
            TextureFormat::Bgr8 => return Some(Self::Bgr8),
            TextureFormat::L8A8 => return Some(Self::L8A8),
            TextureFormat::Dxgi(dxgi) => dxgi.0,
            TextureFormat::UnknownFourCc(_) | TextureFormat::UnknownMasks { .. } => return None,
        };
        Some(match code {
            27..=30 => Self::Rgba8,
            87 | 90 | 91 => Self::Bgra8,
            88 | 92 | 93 => Self::Bgrx8,
            60..=62 => Self::R8,
            65 => Self::A8,
            48..=50 => Self::Rg8,
            53 | 56 | 57 => Self::R16,
            42 => Self::R32,
            33 | 35 | 36 => Self::Rg16,
            9 | 11 | 12 => Self::Rgba16,
            23..=25 => Self::Rgb10A2,
            85 => Self::B5G6R5,
            86 => Self::B5G5R5A1,
            115 => Self::B4G4R4A4,
            63 => Self::R8Snorm,
            51 => Self::Rg8Snorm,
            31 => Self::Rgba8Snorm,
            _ => return None,
        })
    }

    /// Converts one row of pixels `src` into the RGBA row `dst`.
    pub(crate) fn convert_row(self, src: &[u8], dst: &mut [u8]) {
        match self {
            Self::Rgba8 => convert(src, dst, |rgba: [u8; 4]| rgba),
            Self::Bgra8 => convert(src, dst, |[b, g, r, a]| [r, g, b, a]),
            Self::Bgrx8 => convert(src, dst, |[b, g, r, _]| [r, g, b, 255]),
            Self::Bgr8 => convert(src, dst, |[b, g, r]| [r, g, b, 255]),
            Self::L8A8 => convert(src, dst, |[l, a]| [l, l, l, a]),
            Self::R8 => convert(src, dst, |[r]| [r, r, r, 255]),
            Self::A8 => convert(src, dst, |[a]| [0, 0, 0, a]),
            Self::Rg8 => convert(src, dst, |[r, g]| [r, g, 0, 255]),
            Self::R16 => convert(src, dst, |[low, high]| {
                let r = unorm16(low, high);
                [r, r, r, 255]
            }),
            Self::R32 => convert(src, dst, |bytes: [u8; 4]| {
                let r = unorm32(u32::from_le_bytes(bytes));
                [r, r, r, 255]
            }),
            Self::Rg16 => convert(src, dst, |[r0, r1, g0, g1]| {
                [unorm16(r0, r1), unorm16(g0, g1), 0, 255]
            }),
            Self::Rgba16 => convert(src, dst, |[r0, r1, g0, g1, b0, b1, a0, a1]| {
                [
                    unorm16(r0, r1),
                    unorm16(g0, g1),
                    unorm16(b0, b1),
                    unorm16(a0, a1),
                ]
            }),
            Self::Rgb10A2 => convert(src, dst, |bytes| {
                let value = u32::from_le_bytes(bytes);
                [
                    unorm(value, 10),
                    unorm(value >> 10, 10),
                    unorm(value >> 20, 10),
                    unorm(value >> 30, 2),
                ]
            }),
            Self::B5G6R5 | Self::B5G5R5A1 | Self::B4G4R4A4 => self.convert_packed16(src, dst),
            Self::R8Snorm => convert(src, dst, |[r]: [u8; 1]| {
                let r = snorm(r);
                [r, r, r, 255]
            }),
            Self::Rg8Snorm => convert(src, dst, |[r, g]| [snorm(r), snorm(g), 0, 255]),
            Self::Rgba8Snorm => convert(src, dst, |[r, g, b, a]| {
                [snorm(r), snorm(g), snorm(b), snorm_alpha(a)]
            }),
        }
    }

    fn convert_packed16(self, src: &[u8], dst: &mut [u8]) {
        let packed = |bytes: [u8; 2]| u32::from(u16::from_le_bytes(bytes));
        match self {
            Self::B5G6R5 => convert(src, dst, |bytes| {
                let value = packed(bytes);
                [
                    unorm(value >> 11, 5),
                    unorm(value >> 5, 6),
                    unorm(value, 5),
                    255,
                ]
            }),
            Self::B5G5R5A1 => convert(src, dst, |bytes| {
                let value = packed(bytes);
                [
                    unorm(value >> 10, 5),
                    unorm(value >> 5, 5),
                    unorm(value, 5),
                    unorm(value >> 15, 1),
                ]
            }),
            _ => convert(src, dst, |bytes| {
                let value = packed(bytes);
                [
                    unorm(value >> 8, 4),
                    unorm(value >> 4, 4),
                    unorm(value, 4),
                    unorm(value >> 12, 4),
                ]
            }),
        }
    }
}

fn convert<const N: usize>(src: &[u8], dst: &mut [u8], pixel: impl Fn([u8; N]) -> [u8; 4]) {
    let (sources, _) = src.as_chunks::<N>();
    let (targets, _) = dst.as_chunks_mut::<4>();
    for (target, source) in targets.iter_mut().zip(sources) {
        *target = pixel(*source);
    }
}

fn unorm(value: u32, bits: u32) -> u8 {
    let max = (1 << bits) - 1;
    u8::try_from(((value & max) * 255 + max / 2) / max).unwrap_or(u8::MAX)
}

/// Maps a signed byte, -1 to 1, to 0 to 255 as BC4 and BC5 signed do; -128 counts as -127.
fn snorm(value: u8) -> u8 {
    snorm_to_unorm(i32::from(value.cast_signed()))
}

/// Maps the signed alpha byte to 0 to 255; negative values become 0.
fn snorm_alpha(value: u8) -> u8 {
    u8::try_from((i32::from(value.cast_signed()).clamp(0, 127) * 255 + 63) / 127).unwrap_or(u8::MAX)
}

fn unorm32(value: u32) -> u8 {
    let max = u64::from(u32::MAX);
    u8::try_from((u64::from(value) * 255 + max / 2) / max).unwrap_or(u8::MAX)
}

fn unorm16(low: u8, high: u8) -> u8 {
    unorm(u32::from(u16::from_le_bytes([low, high])), 16)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0.5..=255.5 before the cast"
)]
pub(crate) const fn float_unorm(value: f32) -> u8 {
    if value.is_nan() {
        return 0;
    }
    value.clamp(0.0, 1.0).mul_add(255.0, 0.5) as u8
}

/// Converts the bits of a half float to a single float, exactly.
pub(crate) fn half_to_f32(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((bits >> 10) & 0x1F);
    let mantissa = bits & 0x3FF;
    let fraction = f32::from(mantissa);
    let magnitude = match (exponent, mantissa) {
        (0, _) => fraction * 2.0_f32.powi(-24),
        (0x1F, 0) => f32::INFINITY,
        (0x1F, _) => f32::NAN,
        _ => (1.0 + fraction / 1024.0) * 2.0_f32.powi(exponent - 15),
    };
    sign * magnitude
}
