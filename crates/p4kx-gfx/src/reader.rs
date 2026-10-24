// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;

use p4kx_foundation::ParseError;

use crate::GfxError;
use crate::geometry::{Matrix, Rect, Rgba};

/// Reads the byte-aligned and bit-packed fields of one tag body without reading out of bounds.
///
/// Byte reads first skip to the next byte boundary, as the SWF format requires.
#[derive(Debug, Clone)]
pub(crate) struct TagReader<'a> {
    bytes: &'a [u8],
    bit: usize,
    base: usize,
}

impl<'a> TagReader<'a> {
    /// Creates a reader over `bytes`, which start at `base` in the movie data.
    pub(crate) const fn new(bytes: &'a [u8], base: usize) -> Self {
        Self {
            bytes,
            bit: 0,
            base,
        }
    }

    /// Returns the offset of the next whole byte in the movie data.
    pub(crate) const fn offset(&self) -> usize {
        self.base + self.bit.div_ceil(8)
    }

    /// Returns the bytes from the next byte boundary to the end.
    pub(crate) fn rest(&mut self) -> &'a [u8] {
        self.align();
        let rest = self.bytes.get(self.bit / 8..).unwrap_or_default();
        self.bit = self.bytes.len() * 8;
        rest
    }

    /// Returns the number of whole bytes left after the next byte boundary.
    pub(crate) const fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.bit.div_ceil(8))
    }

    /// Skips to the next byte boundary.
    pub(crate) const fn align(&mut self) {
        self.bit = self.bit.div_ceil(8) * 8;
    }

    const fn end_error(&self, needed: usize) -> GfxError {
        GfxError::Parse(ParseError::UnexpectedEnd {
            offset: self.offset() as u64,
            needed: needed as u64,
            available: self.remaining() as u64,
        })
    }

    /// Reads `count` bytes.
    pub(crate) fn bytes(&mut self, count: usize) -> Result<&'a [u8], GfxError> {
        self.align();
        let start = self.bit / 8;
        let slice = start
            .checked_add(count)
            .and_then(|end| self.bytes.get(start..end))
            .ok_or_else(|| self.end_error(count))?;
        self.bit += count * 8;
        Ok(slice)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], GfxError> {
        let mut array = [0; N];
        array.copy_from_slice(self.bytes(N)?);
        Ok(array)
    }

    /// Reads an unsigned byte.
    pub(crate) fn u8(&mut self) -> Result<u8, GfxError> {
        self.array::<1>().map(|[byte]| byte)
    }

    /// Reads a little-endian `u16`.
    pub(crate) fn u16(&mut self) -> Result<u16, GfxError> {
        self.array().map(u16::from_le_bytes)
    }

    /// Reads a little-endian `i16`.
    pub(crate) fn i16(&mut self) -> Result<i16, GfxError> {
        self.array().map(i16::from_le_bytes)
    }

    /// Reads a little-endian `u32`.
    pub(crate) fn u32(&mut self) -> Result<u32, GfxError> {
        self.array().map(u32::from_le_bytes)
    }

    /// Reads an 8.8 fixed-point number.
    pub(crate) fn fixed8(&mut self) -> Result<f32, GfxError> {
        self.i16().map(|value| f32::from(value) / 256.0)
    }

    /// Reads a string ending with a NUL byte, decoded lossily as UTF-8.
    pub(crate) fn c_string(&mut self) -> Result<Cow<'a, str>, GfxError> {
        self.align();
        let rest = self.bytes.get(self.bit / 8..).unwrap_or_default();
        let length = rest
            .iter()
            .position(|&byte| byte == 0)
            .ok_or_else(|| self.end_error(rest.len() + 1))?;
        let text = self.bytes(length)?;
        self.bit += 8;
        Ok(String::from_utf8_lossy(text))
    }

    /// Reads a string of a length byte and that many bytes, decoded lossily as UTF-8 without any
    /// trailing NUL bytes.
    pub(crate) fn short_string(&mut self) -> Result<String, GfxError> {
        let length = usize::from(self.u8()?);
        let text = self.bytes(length)?;
        Ok(String::from_utf8_lossy(text)
            .trim_end_matches('\0')
            .to_owned())
    }

    /// Reads an unsigned bit field of `bits` bits, at most 32.
    pub(crate) fn ub(&mut self, bits: u32) -> Result<u32, GfxError> {
        if bits > 32 {
            return Err(GfxError::BitCount { bits });
        }
        let count = bits as usize;
        let end = self
            .bit
            .checked_add(count)
            .filter(|&end| end <= self.bytes.len() * 8)
            .ok_or_else(|| self.end_error(count.div_ceil(8)))?;
        let mut value = 0_u64;
        let mut position = self.bit;
        while position < end {
            let byte = self.bytes.get(position / 8).copied().unwrap_or_default();
            let used = position % 8;
            let available = 8 - used;
            let take = available.min(end - position);
            let shifted = u64::from(byte) >> (available - take);
            value = (value << take) | (shifted & ((1 << take) - 1));
            position += take;
        }
        self.bit = end;
        u32::try_from(value).map_err(|_| GfxError::BitCount { bits })
    }

    /// Reads a signed bit field of `bits` bits, at most 32.
    pub(crate) fn sb(&mut self, bits: u32) -> Result<i32, GfxError> {
        let value = self.ub(bits)?;
        if bits == 0 {
            return Ok(0);
        }
        let shift = 32 - bits;
        Ok(value.cast_signed().wrapping_shl(shift) >> shift)
    }

    /// Reads a 16.16 fixed-point bit field of `bits` bits.
    pub(crate) fn fb(&mut self, bits: u32) -> Result<f32, GfxError> {
        #[expect(
            clippy::cast_precision_loss,
            reason = "fixed-point values have at most 32 bits"
        )]
        let value = self.sb(bits)? as f32;
        Ok(value / 65_536.0)
    }

    /// Reads one bit as a flag.
    pub(crate) fn flag(&mut self) -> Result<bool, GfxError> {
        self.ub(1).map(|bit| bit == 1)
    }

    /// Reads a RECT.
    pub(crate) fn rect(&mut self) -> Result<Rect, GfxError> {
        self.align();
        let bits = self.ub(5)?;
        let rect = Rect {
            x_min: self.sb(bits)?,
            x_max: self.sb(bits)?,
            y_min: self.sb(bits)?,
            y_max: self.sb(bits)?,
        };
        self.align();
        Ok(rect)
    }

    /// Reads a MATRIX.
    pub(crate) fn matrix(&mut self) -> Result<Matrix, GfxError> {
        self.align();
        let mut matrix = Matrix::IDENTITY;
        if self.flag()? {
            let bits = self.ub(5)?;
            matrix.scale_x = self.fb(bits)?;
            matrix.scale_y = self.fb(bits)?;
        }
        if self.flag()? {
            let bits = self.ub(5)?;
            matrix.rotate_skew0 = self.fb(bits)?;
            matrix.rotate_skew1 = self.fb(bits)?;
        }
        let bits = self.ub(5)?;
        matrix.translate_x = self.sb(bits)?;
        matrix.translate_y = self.sb(bits)?;
        self.align();
        Ok(matrix)
    }

    /// Reads an RGB colour as opaque.
    pub(crate) fn rgb(&mut self) -> Result<Rgba, GfxError> {
        let [red, green, blue] = self.array()?;
        Ok(Rgba {
            red,
            green,
            blue,
            alpha: u8::MAX,
        })
    }

    /// Reads an RGBA colour.
    pub(crate) fn rgba(&mut self) -> Result<Rgba, GfxError> {
        let [red, green, blue, alpha] = self.array()?;
        Ok(Rgba {
            red,
            green,
            blue,
            alpha,
        })
    }

    /// Reads an RGBA colour when `alpha` is set, otherwise an opaque RGB colour.
    pub(crate) fn color(&mut self, alpha: bool) -> Result<Rgba, GfxError> {
        if alpha { self.rgba() } else { self.rgb() }
    }

    /// Reads a style count: one byte, or a `u16` after `0xFF` when `extended` is set.
    pub(crate) fn count(&mut self, extended: bool) -> Result<usize, GfxError> {
        let count = self.u8()?;
        if count == u8::MAX && extended {
            Ok(usize::from(self.u16()?))
        } else {
            Ok(usize::from(count))
        }
    }
}
