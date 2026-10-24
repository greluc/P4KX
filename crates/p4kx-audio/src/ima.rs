// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Wwise IMA ADPCM block decoder: XBOX IMA with one sub-block per channel.

use crate::ByteOrder;
use crate::adpcm::{clamp16, nibbles, sample_bytes};

/// Bytes of the header in front of every channel's sub-block.
pub(crate) const HEADER_LEN: usize = 4;

const MAX_STEP_INDEX: i32 = 88;

const STEP_SIZES: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

const INDEX_STEPS: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

/// Returns the samples a sub-block of `len` bytes holds: the header sample and every nibble but
/// the last.
pub(crate) const fn samples_per_sub_block(len: usize) -> usize {
    (len - HEADER_LEN) * 2
}

/// Decodes one channel's sub-block into `out`, one sample per slot.
///
/// The sub-block must be longer than [`HEADER_LEN`].
pub(crate) fn decode<'a>(
    sub_block: &[u8],
    order: ByteOrder,
    mut out: impl Iterator<Item = &'a mut [u8; 2]>,
) {
    let mut history = order
        .u16_at(sub_block, 0)
        .map_or(0, |sample| i32::from(sample.cast_signed()));
    let mut step_index = sub_block
        .get(2)
        .map_or(0, |&index| i32::from(index.cast_signed()))
        .clamp(0, MAX_STEP_INDEX);
    if let Some(first) = out.next() {
        *first = sample_bytes(history);
    }
    for (slot, nibble) in out.zip(nibbles(sub_block, HEADER_LEN)) {
        let step = usize::try_from(step_index)
            .ok()
            .and_then(|index| STEP_SIZES.get(index))
            .copied()
            .unwrap_or(0);
        let magnitude = nibble & 0x07;
        let delta = ((i32::from(magnitude) * 2 + 1) * step) >> 3;
        history = clamp16(history + if nibble & 0x08 == 0 { delta } else { -delta });
        let index_step = INDEX_STEPS
            .get(usize::from(magnitude))
            .copied()
            .unwrap_or(0);
        step_index = (step_index + index_step).clamp(0, MAX_STEP_INDEX);
        *slot = sample_bytes(history);
    }
}
