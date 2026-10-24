// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The RIFF chunks, format tags and padding of the WEMs of a real Archive.

use std::collections::BTreeMap;

const RIFF_HEADER_LEN: usize = 12;
const CHUNK_HEADER_LEN: usize = 8;
const DSP_HEADER_LEN: usize = 0x0C;
const DSP_CHANNEL_LEN: usize = 0x2E;
const PTADPCM_FRAME_LEN: usize = 0x104;
const IMA_TAG: u16 = 2;

#[derive(Default)]
pub(crate) struct WemSurvey {
    pub(crate) wems: usize,
    pub(crate) tags: BTreeMap<String, usize>,
    pub(crate) after_data: BTreeMap<String, Vec<String>>,
    pub(crate) odd_not_last: Vec<String>,
    pub(crate) odd_padded: usize,
    pub(crate) odd_unpadded: usize,
    pub(crate) odd_neither: usize,
}

impl WemSurvey {
    pub(crate) fn merge(mut self, other: Self) -> Self {
        self.wems += other.wems;
        for (tag, count) in other.tags {
            *self.tags.entry(tag).or_default() += count;
        }
        for (id, paths) in other.after_data {
            self.after_data.entry(id).or_default().extend(paths);
        }
        self.odd_not_last.extend(other.odd_not_last);
        self.odd_padded += other.odd_padded;
        self.odd_unpadded += other.odd_unpadded;
        self.odd_neither += other.odd_neither;
        self
    }

    pub(crate) fn add(&mut self, label: &str, bytes: &[u8]) {
        let Some(riff) = Riff::of(bytes) else {
            return;
        };
        self.wems += 1;
        let end = riff
            .u32(4)
            .map_or(0, |size| size.saturating_add(8))
            .min(bytes.len());
        let mut position = RIFF_HEADER_LEN;
        let mut seen_data = false;
        while position + CHUNK_HEADER_LEN <= end {
            let Some(id) = bytes.get(position..position + 4) else {
                break;
            };
            let size = riff.u32(position + 4).unwrap_or(0);
            let body = position + CHUNK_HEADER_LEN;
            if id == b"fmt " {
                *self.tags.entry(riff.tag_name(body)).or_default() += 1;
            } else if seen_data {
                self.after_data
                    .entry(String::from_utf8_lossy(id).into_owned())
                    .or_default()
                    .push(label.to_owned());
            }
            seen_data |= id == b"data";
            let Some(next) = body.checked_add(size) else {
                break;
            };
            position = if size % 2 == 1 && next + CHUNK_HEADER_LEN < end {
                self.count_odd(&riff, label, next)
            } else {
                next
            };
        }
    }

    fn count_odd(&mut self, riff: &Riff<'_>, label: &str, next: usize) -> usize {
        self.odd_not_last.push(label.to_owned());
        let padded = riff.valid_header(next + 1);
        let unpadded = riff.valid_header(next);
        self.odd_padded += usize::from(padded && !unpadded);
        self.odd_unpadded += usize::from(unpadded && !padded);
        self.odd_neither += usize::from(!padded && !unpadded);
        if padded { next + 1 } else { next }
    }
}

struct Riff<'a> {
    bytes: &'a [u8],
    big: bool,
}

impl<'a> Riff<'a> {
    fn of(bytes: &'a [u8]) -> Option<Self> {
        match bytes.get(..4)? {
            b"RIFF" => Some(Self { bytes, big: false }),
            b"RIFX" => Some(Self { bytes, big: true }),
            _ => None,
        }
    }

    fn u16(&self, offset: usize) -> Option<usize> {
        let field = *self.bytes.get(offset..)?.first_chunk::<2>()?;
        Some(usize::from(if self.big {
            u16::from_be_bytes(field)
        } else {
            u16::from_le_bytes(field)
        }))
    }

    fn u32(&self, offset: usize) -> Option<usize> {
        let field = *self.bytes.get(offset..)?.first_chunk::<4>()?;
        usize::try_from(if self.big {
            u32::from_be_bytes(field)
        } else {
            u32::from_le_bytes(field)
        })
        .ok()
    }

    fn valid_header(&self, at: usize) -> bool {
        let printable = self
            .bytes
            .get(at..at + 4)
            .is_some_and(|id| id.iter().all(|byte| (0x20..0x7F).contains(byte)));
        printable
            && self
                .u32(at + 4)
                .is_some_and(|size| at + CHUNK_HEADER_LEN + size <= self.bytes.len())
    }

    fn tag_name(&self, fmt: usize) -> String {
        let tag = self.u16(fmt).unwrap_or(0);
        if tag != usize::from(IMA_TAG) {
            return format!("{tag:#06x}");
        }
        let channels = self.u16(fmt + 2).unwrap_or(0);
        let block_align = self.u16(fmt + 0x0C).unwrap_or(0);
        let extra = self.u16(fmt + 0x10).unwrap_or(0);
        if extra == DSP_HEADER_LEN + channels * DSP_CHANNEL_LEN {
            "0x0002 dsp"
        } else if block_align == PTADPCM_FRAME_LEN * channels {
            "0x0002 ptadpcm"
        } else {
            "0x0002 ima"
        }
        .to_owned()
    }
}
