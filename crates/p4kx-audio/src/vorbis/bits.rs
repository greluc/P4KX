// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// Reads bit fields least significant bit first, as Vorbis packs them, from a byte slice.
#[derive(Debug, Clone)]
pub(crate) struct BitReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> BitReader<'a> {
    pub(crate) const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    /// Reads a field of up to 32 bits; `None` when the bytes end first.
    pub(crate) fn read(&mut self, bits: u32) -> Option<u32> {
        let mut value = 0_u32;
        for shift in 0..bits.min(32) {
            let byte = *self.bytes.get(self.position / 8)?;
            let bit = (byte >> (self.position % 8)) & 1;
            value |= u32::from(bit) << shift;
            self.position += 1;
        }
        Some(value)
    }

    /// Returns the number of bits read so far.
    pub(crate) const fn bits_read(&self) -> usize {
        self.position
    }
}

/// Appends bit fields least significant bit first to a byte buffer the caller owns.
#[derive(Debug)]
pub(crate) struct BitWriter<'a> {
    out: &'a mut Vec<u8>,
    pending: u64,
    filled: u32,
}

impl<'a> BitWriter<'a> {
    pub(crate) const fn new(out: &'a mut Vec<u8>) -> Self {
        Self {
            out,
            pending: 0,
            filled: 0,
        }
    }

    /// Appends the low `bits` bits of `value`, at most 32.
    pub(crate) fn write(&mut self, value: u32, bits: u32) {
        let bits = bits.min(32);
        let mask = (1_u64 << bits) - 1;
        self.pending |= (u64::from(value) & mask) << self.filled;
        self.filled += bits;
        while self.filled >= 8 {
            self.out.push(self.pending.to_le_bytes()[0]);
            self.pending >>= 8;
            self.filled -= 8;
        }
    }

    /// Writes the bytes of `bytes` as whole 8-bit fields.
    pub(crate) fn write_bytes(&mut self, bytes: &[u8]) {
        if self.filled == 0 {
            self.out.extend_from_slice(bytes);
        } else {
            for &byte in bytes {
                self.write(u32::from(byte), 8);
            }
        }
    }

    /// Pads the last byte with zero bits.
    pub(crate) fn finish(self) {
        if self.filled > 0 {
            self.out.push(self.pending.to_le_bytes()[0]);
        }
    }
}

/// Copies a field of `bits` bits; `None` when the input ends first.
pub(crate) fn copy(input: &mut BitReader<'_>, out: &mut BitWriter<'_>, bits: u32) -> Option<u32> {
    let value = input.read(bits)?;
    out.write(value, bits);
    Some(value)
}

/// Returns the number of bits needed to store `value`, the Vorbis `ilog`.
pub(crate) const fn ilog(value: u32) -> u32 {
    value.bit_width()
}
