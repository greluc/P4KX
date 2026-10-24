// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::ByteOrder;

const LOOP_COUNT_OFFSET: usize = 0x1C;
const LOOPS_OFFSET: usize = 0x24;
const LOOP_LEN: usize = 0x18;
const LOOP_KIND_OFFSET: usize = 0x04;
const LOOP_START_OFFSET: usize = 0x08;
const LOOP_END_OFFSET: usize = 0x0C;
const FORWARD: u32 = 0;
const UNITY_NOTE: u32 = 60;
const NANOS_PER_SECOND: u32 = 1_000_000_000;
const SMPL_BODY_LEN: usize = LOOPS_OFFSET + LOOP_LEN;

/// The length of the `smpl` chunk [`Loop::smpl_chunk`] writes, header included.
pub(crate) const SMPL_CHUNK_LEN: usize = 8 + SMPL_BODY_LEN;

/// The comments [`Loop::write_comments`] writes.
pub(crate) const LOOP_COMMENTS: u32 = 2;

/// The loop of a looping sound in frames: it repeats from `start` up to, not including, `end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Loop {
    /// The first frame of the loop.
    pub start: u64,
    /// The frame after the last frame of the loop.
    pub end: u64,
}

impl Loop {
    /// Returns the frames one pass of the loop plays.
    #[must_use]
    pub const fn frames(self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    /// Returns whether the loop is non-empty and ends within `frames` frames.
    #[must_use]
    pub const fn fits(self, frames: u64) -> bool {
        self.start < self.end && self.end <= frames
    }

    /// Returns the first forward loop of a `smpl` chunk body that fits `sample_count` frames;
    /// `None` without one, for a truncated chunk or an unknown sample count.
    pub(crate) fn from_smpl(
        order: ByteOrder,
        smpl: &[u8],
        sample_count: Option<u64>,
    ) -> Option<Self> {
        let frames = sample_count?;
        let count = usize::try_from(order.u32_at(smpl, LOOP_COUNT_OFFSET)?).ok()?;
        smpl.get(LOOPS_OFFSET..)?
            .as_chunks::<LOOP_LEN>()
            .0
            .iter()
            .take(count)
            .find_map(|entry| {
                if order.u32_at(entry, LOOP_KIND_OFFSET)? != FORWARD {
                    return None;
                }
                let start = u64::from(order.u32_at(entry, LOOP_START_OFFSET)?);
                let end = match order.u32_at(entry, LOOP_END_OFFSET)? {
                    0 => frames,
                    last => u64::from(last) + 1,
                };
                Some(Self { start, end }).filter(|found| found.fits(frames))
            })
    }

    /// Returns a little-endian `smpl` chunk holding the loop as one infinite forward loop with
    /// an inclusive end; `None` when a loop point exceeds 32 bits or the loop is empty.
    pub(crate) fn smpl_chunk(self, sample_rate: u32) -> Option<[u8; SMPL_CHUNK_LEN]> {
        let start = u32::try_from(self.start).ok()?;
        let last = u32::try_from(self.end.checked_sub(1)?).ok()?;
        if start > last {
            return None;
        }
        let period = NANOS_PER_SECOND
            .checked_div(sample_rate)
            .unwrap_or_default();
        let body_len = u32::try_from(SMPL_BODY_LEN).ok()?;
        let fields = [
            body_len, 0, 0, period, UNITY_NOTE, 0, 0, 0, 1, 0, 0, FORWARD, start, last, 0, 0,
        ];
        let mut chunk = [0; SMPL_CHUNK_LEN];
        chunk.first_chunk_mut::<4>()?.copy_from_slice(b"smpl");
        let (words, _) = chunk.get_mut(4..)?.as_chunks_mut::<4>();
        for (word, field) in words.iter_mut().zip(fields) {
            *word = field.to_le_bytes();
        }
        Some(chunk)
    }

    /// Appends the `LOOPSTART` and `LOOPLENGTH` comments, each with its little-endian length, as
    /// Vorbis comment headers and `OpusTags` hold them.
    pub(crate) fn write_comments(self, out: &mut Vec<u8>) {
        for (key, value) in [("LOOPSTART=", self.start), ("LOOPLENGTH=", self.frames())] {
            let value = value.to_string();
            let len = u32::try_from(key.len() + value.len()).unwrap_or_default();
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(key.as_bytes());
            out.extend_from_slice(value.as_bytes());
        }
    }
}

/// Adds the comments of `points` to a comment packet whose vendor string starts at `vendor`;
/// leaves a malformed packet unchanged and returns whether the comments were added.
pub(crate) fn insert_comments(packet: &mut Vec<u8>, vendor: usize, points: Loop) -> bool {
    let u32_at = |bytes: &[u8], offset: usize| ByteOrder::Little.u32_at(bytes, offset);
    let field = |offset: usize| usize::try_from(u32_at(packet, offset)?).ok();
    let Some(count_at) = field(vendor).and_then(|len| vendor.checked_add(4)?.checked_add(len))
    else {
        return false;
    };
    let Some(count) = u32_at(packet, count_at) else {
        return false;
    };
    let mut end = count_at + 4;
    for _ in 0..count {
        let Some(next) = field(end).and_then(|len| end.checked_add(4)?.checked_add(len)) else {
            return false;
        };
        if next > packet.len() {
            return false;
        }
        end = next;
    }
    let Some(total) = count.checked_add(LOOP_COMMENTS) else {
        return false;
    };
    let mut comments = Vec::new();
    points.write_comments(&mut comments);
    packet.splice(end..end, comments);
    if let Some(slot) = packet.get_mut(count_at..count_at + 4) {
        slot.copy_from_slice(&total.to_le_bytes());
    }
    true
}
