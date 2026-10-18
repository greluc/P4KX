use crate::bc::Texels;
use crate::bc6h_modes::{FIELDS, Mode, PARTITION, mode_of};
use crate::bc7_tables::{ANCHORS, SUBSETS, WEIGHTS3, WEIGHTS4};
use crate::pixels::half_to_f32;
use crate::tone_map::tone_map;

/// The 16 texels of one decoded BC6H block, the half-float bits of red, green and blue, row by
/// row.
pub(crate) type HalfTexels = [[u16; 3]; 16];

/// The bits of a block not read yet, the next one lowest.
struct Bits(u128);

impl Bits {
    fn take(&mut self, count: u32) -> i32 {
        let value = self.0 & ((1 << count) - 1);
        self.0 = self.0.checked_shr(count).unwrap_or_default();
        i32::try_from(value).unwrap_or_default()
    }
}

/// Decodes an unsigned BC6H block to half floats; a reserved mode decodes to zero.
pub(crate) fn bc6h_unsigned(block: [u8; 16]) -> HalfTexels {
    bc6h(block, false)
}

/// Decodes a signed BC6H block to half floats; a reserved mode decodes to zero.
pub(crate) fn bc6h_signed(block: [u8; 16]) -> HalfTexels {
    bc6h(block, true)
}

/// Decodes an unsigned BC6H block and tone-maps it to opaque 8-bit sRGB.
pub(crate) fn bc6h_unsigned_rgba8(block: [u8; 16]) -> Texels {
    to_rgba8(bc6h_unsigned(block))
}

/// Decodes a signed BC6H block and tone-maps it to opaque 8-bit sRGB.
pub(crate) fn bc6h_signed_rgba8(block: [u8; 16]) -> Texels {
    to_rgba8(bc6h_signed(block))
}

fn to_rgba8(halves: HalfTexels) -> Texels {
    halves.map(|[r, g, b]| {
        let [r, g, b] = [r, g, b].map(|half| tone_map(half_to_f32(half)));
        [r, g, b, u8::MAX]
    })
}

fn bc6h(block: [u8; 16], signed: bool) -> HalfTexels {
    let raw = u128::from_le_bytes(block);
    let Some(mode) = mode_of(raw) else {
        return [[0; 3]; 16];
    };
    let mut bits = Bits(raw >> mode.number_bits);
    let mut fields = [0_i32; FIELDS];
    for run in mode.runs {
        let value = bits.take(u32::from(run.len));
        if let Some(field) = fields.get_mut(usize::from(run.field)) {
            *field |= value << run.shift;
        }
    }
    let endpoints = endpoints(mode, &fields, signed);
    let (subsets, anchors, index_bits, weights): (u32, u16, u32, &[u8]) = if mode.two_regions {
        let partition = fields
            .get(PARTITION)
            .and_then(|&number| usize::try_from(number).ok())
            .unwrap_or_default();
        let subsets = SUBSETS.get(1).and_then(|table| table.get(partition));
        let anchors = ANCHORS.get(1).and_then(|table| table.get(partition));
        (
            subsets.copied().unwrap_or_default(),
            anchors.copied().unwrap_or(1),
            3,
            &WEIGHTS3,
        )
    } else {
        (0, 1, 4, &WEIGHTS4)
    };
    let mut texels = [[0; 3]; 16];
    for (texel, out) in (0_u32..).zip(texels.iter_mut()) {
        let subset = usize::try_from((subsets >> (2 * texel)) & 0x3).unwrap_or_default();
        let anchor = u32::from((anchors >> texel) & 1);
        let index = usize::try_from(bits.take(index_bits - anchor)).unwrap_or_default();
        let weight = i32::from(weights.get(index).copied().unwrap_or_default());
        let pair = endpoints.get(2 * subset..).unwrap_or_default();
        let (Some(first), Some(second)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        for ((value, &a), &b) in out.iter_mut().zip(first).zip(second) {
            *value = finish_unquantize(((64 - weight) * a + weight * b + 32) >> 6, signed);
        }
    }
    texels
}

/// Returns the unquantized endpoints W, X, Y and Z, each red, green and blue.
fn endpoints(mode: &Mode, fields: &[i32; FIELDS], signed: bool) -> [[i32; 3]; 4] {
    let precision = mode.endpoint_bits;
    let count = if mode.two_regions { 4 } else { 2 };
    let (stored, _) = fields.as_chunks::<3>();
    let base = stored.first().copied().unwrap_or_default();
    let mut out = [[0; 3]; 4];
    for (index, (endpoint, raw)) in out.iter_mut().zip(stored).take(count).enumerate() {
        for (channel, value) in endpoint.iter_mut().enumerate() {
            let raw = raw.get(channel).copied().unwrap_or_default();
            let w = base.get(channel).copied().unwrap_or_default();
            let w = if signed { extend_sign(w, precision) } else { w };
            let delta_bits = mode.delta_bits.get(channel).copied().unwrap_or(precision);
            let quantized = if index == 0 {
                w
            } else if mode.transformed {
                let sum = (w + extend_sign(raw, delta_bits)) & ((1 << precision) - 1);
                if signed {
                    extend_sign(sum, precision)
                } else {
                    sum
                }
            } else if signed {
                extend_sign(raw, precision)
            } else {
                raw
            };
            *value = unquantize(quantized, precision, signed);
        }
    }
    out
}

const fn extend_sign(value: i32, bits: u32) -> i32 {
    let shift = 32 - bits;
    value.wrapping_shl(shift).wrapping_shr(shift)
}

/// Scales an endpoint of `bits` bits to the full 16-bit range.
const fn unquantize(value: i32, bits: u32, signed: bool) -> i32 {
    if signed {
        if bits >= 16 {
            return value;
        }
        let magnitude = value.abs();
        let scaled = if magnitude == 0 {
            0
        } else if magnitude >= (1 << (bits - 1)) - 1 {
            0x7FFF
        } else {
            ((magnitude << 15) + 0x4000) >> (bits - 1)
        };
        if value < 0 { -scaled } else { scaled }
    } else if bits >= 15 {
        value
    } else if value == 0 {
        0
    } else if value == (1 << bits) - 1 {
        0xFFFF
    } else {
        ((value << 16) + 0x8000) >> bits
    }
}

/// Scales an interpolated value to the bits of a half float.
fn finish_unquantize(value: i32, signed: bool) -> u16 {
    let magnitude = if signed {
        (value.unsigned_abs() * 31) >> 5
    } else {
        (value.unsigned_abs() * 31) >> 6
    };
    let sign = if signed && value < 0 && magnitude != 0 {
        0x8000
    } else {
        0
    };
    u16::try_from(magnitude).map_or(0x7C00 | sign, |magnitude| magnitude | sign)
}
