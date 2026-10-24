// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::bc::Texels;
use crate::bc7_tables::{ANCHORS, SUBSETS, WEIGHTS2, WEIGHTS3, WEIGHTS4};

/// The field widths of one BC7 mode.
#[derive(Debug, Clone, Copy)]
struct Mode {
    subsets: usize,
    partition_bits: u32,
    rotation_bits: u32,
    selector_bits: u32,
    colour_bits: u32,
    alpha_bits: u32,
    p_bits_per_subset: usize,
    index_bits: u32,
    index2_bits: u32,
}

const ONE_SUBSET: Mode = Mode {
    subsets: 1,
    partition_bits: 0,
    rotation_bits: 0,
    selector_bits: 0,
    colour_bits: 0,
    alpha_bits: 0,
    p_bits_per_subset: 0,
    index_bits: 2,
    index2_bits: 0,
};

/// The field widths of modes 0 to 7.
const MODES: [Mode; 8] = [
    Mode {
        subsets: 3,
        partition_bits: 4,
        colour_bits: 4,
        p_bits_per_subset: 2,
        index_bits: 3,
        ..ONE_SUBSET
    },
    Mode {
        subsets: 2,
        partition_bits: 6,
        colour_bits: 6,
        p_bits_per_subset: 1,
        index_bits: 3,
        ..ONE_SUBSET
    },
    Mode {
        subsets: 3,
        partition_bits: 6,
        colour_bits: 5,
        ..ONE_SUBSET
    },
    Mode {
        subsets: 2,
        partition_bits: 6,
        colour_bits: 7,
        p_bits_per_subset: 2,
        ..ONE_SUBSET
    },
    Mode {
        rotation_bits: 2,
        selector_bits: 1,
        colour_bits: 5,
        alpha_bits: 6,
        index2_bits: 3,
        ..ONE_SUBSET
    },
    Mode {
        rotation_bits: 2,
        colour_bits: 7,
        alpha_bits: 8,
        index2_bits: 2,
        ..ONE_SUBSET
    },
    Mode {
        colour_bits: 7,
        alpha_bits: 7,
        p_bits_per_subset: 2,
        index_bits: 4,
        ..ONE_SUBSET
    },
    Mode {
        subsets: 2,
        partition_bits: 6,
        colour_bits: 5,
        alpha_bits: 5,
        p_bits_per_subset: 2,
        ..ONE_SUBSET
    },
];

const MAX_ENDPOINTS: usize = 6;

/// The bits of a block not read yet, the next one lowest.
struct Bits(u128);

impl Bits {
    fn take(&mut self, count: u32) -> u8 {
        let value = self.0 & ((1 << count) - 1);
        self.0 >>= count;
        u8::try_from(value).unwrap_or_default()
    }
}

/// Decodes a BC7 block; a block without a valid mode is transparent black.
pub(crate) fn bc7(block: [u8; 16]) -> Texels {
    let raw = u128::from_le_bytes(block);
    let mode_number = raw.trailing_zeros();
    let Some(mode) = usize::try_from(mode_number)
        .ok()
        .and_then(|index| MODES.get(index))
    else {
        return [[0; 4]; 16];
    };
    let mut bits = Bits(raw >> (mode_number + 1));
    let partition = usize::from(bits.take(mode.partition_bits));
    let rotation = bits.take(mode.rotation_bits);
    let selector = bits.take(mode.selector_bits);
    let endpoints = endpoints(&mut bits, mode);
    let subsets = lookup(&SUBSETS, mode, partition).unwrap_or_default();
    let anchors = lookup(&ANCHORS, mode, partition).unwrap_or(1);
    let primary = indices(&mut bits, mode.index_bits, anchors);
    let secondary = indices(&mut bits, mode.index2_bits, 1);
    let ((colour, colour_bits), (alpha, alpha_bits)) = match (mode.index2_bits, selector) {
        (0, _) => ((primary, mode.index_bits), (primary, mode.index_bits)),
        (_, 0) => ((primary, mode.index_bits), (secondary, mode.index2_bits)),
        _ => ((secondary, mode.index2_bits), (primary, mode.index_bits)),
    };
    let (colour_weights, alpha_weights) = (weights(colour_bits), weights(alpha_bits));
    let mut texels = [[0; 4]; 16];
    for (texel, ((out, colour), alpha)) in texels.iter_mut().zip(colour).zip(alpha).enumerate() {
        let subset = usize::try_from((subsets >> (2 * texel)) & 0x3).unwrap_or_default();
        let pair = endpoints.get(2 * subset..).unwrap_or_default();
        let (Some(first), Some(second)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        let weight = |weights: &[u8], index: u8| {
            weights.get(usize::from(index)).copied().unwrap_or_default()
        };
        let colour_weight = weight(colour_weights, colour);
        let channel_weights = [
            colour_weight,
            colour_weight,
            colour_weight,
            weight(alpha_weights, alpha),
        ];
        let mut rgba = [0; 4];
        for (((value, &a), &b), weight) in
            rgba.iter_mut().zip(first).zip(second).zip(channel_weights)
        {
            *value = interpolate(a, b, weight);
        }
        *out = rotate(rgba, rotation);
    }
    texels
}

fn endpoints(bits: &mut Bits, mode: &Mode) -> [[u8; 4]; MAX_ENDPOINTS] {
    let count = 2 * mode.subsets;
    let mut endpoints = [[0, 0, 0, 255]; MAX_ENDPOINTS];
    for channel in 0..3 {
        for endpoint in endpoints.iter_mut().take(count) {
            set(endpoint, channel, bits.take(mode.colour_bits));
        }
    }
    if mode.alpha_bits > 0 {
        for endpoint in endpoints.iter_mut().take(count) {
            set(endpoint, 3, bits.take(mode.alpha_bits));
        }
    }
    let per_subset = mode.p_bits_per_subset;
    let mut p_bits = [0; MAX_ENDPOINTS];
    for p_bit in p_bits.iter_mut().take(mode.subsets * per_subset) {
        *p_bit = bits.take(1);
    }
    let has_p_bit = u32::from(per_subset > 0);
    for (index, endpoint) in endpoints.iter_mut().take(count).enumerate() {
        let p_bit = p_bits
            .get(index / 2 * per_subset + index % 2 * per_subset / 2)
            .copied()
            .unwrap_or_default();
        let precision = [
            mode.colour_bits,
            mode.colour_bits,
            mode.colour_bits,
            mode.alpha_bits,
        ];
        for (value, bits) in endpoint.iter_mut().zip(precision) {
            if bits > 0 {
                *value = expand((*value << has_p_bit) | (p_bit & 1), bits + has_p_bit);
            }
        }
    }
    endpoints
}

fn lookup<T: Copy>(table: &[[T; 64]; 3], mode: &Mode, partition: usize) -> Option<T> {
    table
        .get(mode.subsets.checked_sub(1)?)?
        .get(partition)
        .copied()
}

fn set(endpoint: &mut [u8; 4], channel: usize, value: u8) {
    if let Some(slot) = endpoint.get_mut(channel) {
        *slot = value;
    }
}

fn expand(value: u8, precision: u32) -> u8 {
    let (Some(up), Some(down)) = (
        8_u32.checked_sub(precision),
        precision
            .checked_mul(2)
            .and_then(|bits| bits.checked_sub(8)),
    ) else {
        return value;
    };
    let wide = u16::from(value);
    let full =
        wide.checked_shl(up).unwrap_or_default() | wide.checked_shr(down).unwrap_or_default();
    u8::try_from(full & 0xFF).unwrap_or_default()
}

fn indices(bits: &mut Bits, count: u32, anchors: u16) -> [u8; 16] {
    let mut indices = [0; 16];
    if count == 0 {
        return indices;
    }
    for (texel, index) in indices.iter_mut().enumerate() {
        let anchor = u32::from((anchors >> texel) & 1);
        *index = bits.take(count - anchor);
    }
    indices
}

const fn weights(bits: u32) -> &'static [u8] {
    match bits {
        2 => &WEIGHTS2,
        3 => &WEIGHTS3,
        4 => &WEIGHTS4,
        _ => &[0],
    }
}

fn interpolate(first: u8, second: u8, weight: u8) -> u8 {
    let (a, b, w) = (
        u16::from(first),
        u16::from(second),
        u16::from(weight.min(64)),
    );
    u8::try_from(((64 - w) * a + w * b + 32) >> 6).unwrap_or(u8::MAX)
}

const fn rotate(rgba: [u8; 4], rotation: u8) -> [u8; 4] {
    let [r, g, b, a] = rgba;
    match rotation {
        1 => [a, g, b, r],
        2 => [r, a, b, g],
        3 => [r, g, a, b],
        _ => rgba,
    }
}
