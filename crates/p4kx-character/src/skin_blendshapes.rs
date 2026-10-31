// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The `BShapesGpu` chunk of the `.skinm` of a head: the blendshapes of the head, stored per
//! vertex for the GPU.

use thiserror::Error;

use crate::rig_blend::{ExpressionTarget, NEGLIGIBLE};
use crate::rig_bytes::{u32_at, vec3_at};

/// The 32 bytes the chunk starts with: eight `u32`. Words 0 to 2 are the numbers of blendshapes,
/// vertices and entries; words 3 to 6 the offsets of the tables below, in this order; word 7
/// the offset of the pool of blendshape names, which this reader does not use.
const HEADER_LEN: usize = 32;
const WORD_BLENDSHAPES: usize = 0;
const WORD_VERTICES: usize = 1;
const WORD_ENTRIES: usize = 2;
const WORD_STARTS: usize = 3;
const WORD_RANGES: usize = 4;
const WORD_BITMAPS: usize = 5;
const WORD_CODES: usize = 6;

/// A vertex has a bitmap of this many bytes: one bit per blendshape.
const BITMAP_LEN: usize = 64;
/// The most blendshapes a bitmap can address.
pub const MAX_SKIN_BLENDSHAPES: usize = BITMAP_LEN * 8;
/// A blendshape has a range of six `f32`: the scale and the bias of its codes on x, y and z.
const RANGE_LEN: usize = 24;
const BIAS_AT: usize = 12;
/// An entry is one code per axis.
const CODE_LEN: usize = 3;

/// A failure to read the `BShapesGpu` chunk of a head.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum SkinBlendshapesError {
    /// The chunk is shorter than its header.
    #[error("the BShapesGpu chunk is truncated: {have} bytes, the header needs {need}")]
    Truncated {
        /// The bytes the header needs.
        need: usize,
        /// The bytes the chunk has.
        have: usize,
    },
    /// The chunk breaks its layout.
    #[error("the BShapesGpu chunk is malformed: {0}")]
    Malformed(&'static str),
}

/// The blendshapes of a head as its skin geometry stores them.
///
/// Every vertex has a bitmap of the blendshapes that move it and, for each of those in the order
/// of their numbers, three byte codes that a per-blendshape scale and bias turn into the move in
/// metres. These are the blendshapes of the head itself, the cells of the base head of a DNA rig; a head
/// that ships no DNA rig, like a story character's own, has no other source. The reader borrows
/// the chunk, checks every table against it and decodes on demand.
#[derive(Debug, Clone, Copy)]
pub struct SkinBlendshapes<'a> {
    blendshapes: usize,
    starts: &'a [u8],
    ranges: &'a [u8],
    bitmaps: &'a [u8],
    codes: &'a [u8],
}

impl<'a> SkinBlendshapes<'a> {
    /// Reads the tables of the `BShapesGpu` chunk `chunk`.
    ///
    /// # Errors
    ///
    /// Returns [`SkinBlendshapesError`] when the chunk is shorter than its header, a table lies
    /// outside the chunk, or the bitmaps of the vertices do not count the entries the vertices
    /// start.
    pub fn read(chunk: &'a [u8]) -> Result<Self, SkinBlendshapesError> {
        let malformed = SkinBlendshapesError::Malformed;
        if chunk.len() < HEADER_LEN {
            return Err(SkinBlendshapesError::Truncated {
                need: HEADER_LEN,
                have: chunk.len(),
            });
        }
        let word = |index: usize| {
            u32_at(chunk, index * 4)
                .and_then(|value| usize::try_from(value).ok())
                .unwrap_or(usize::MAX)
        };
        let (blendshapes, vertices, entries) = (
            word(WORD_BLENDSHAPES),
            word(WORD_VERTICES),
            word(WORD_ENTRIES),
        );
        if blendshapes == 0 || blendshapes > MAX_SKIN_BLENDSHAPES {
            return Err(malformed("the number of blendshapes is not 1 to 512"));
        }
        let table = |word_index: usize, count: usize, size: usize| {
            let start = word(word_index);
            let end = count.checked_mul(size)?.checked_add(start)?;
            chunk.get(start..end)
        };
        let tables = (
            table(WORD_STARTS, vertices, 4),
            table(WORD_RANGES, blendshapes, RANGE_LEN),
            table(WORD_BITMAPS, vertices, BITMAP_LEN),
            table(WORD_CODES, entries, CODE_LEN),
        );
        let (Some(starts), Some(ranges), Some(bitmaps), Some(codes)) = tables else {
            return Err(malformed("a table lies outside the chunk"));
        };
        let this = Self {
            blendshapes,
            starts,
            ranges,
            bitmaps,
            codes,
        };
        this.check(entries)?;
        Ok(this)
    }

    /// Checks the bitmap of every vertex against the entries the starts give it. The first start
    /// need not be 0: only the difference to the next start counts.
    fn check(&self, entries: usize) -> Result<(), SkinBlendshapesError> {
        let malformed = SkinBlendshapesError::Malformed;
        for (vertex, bitmap) in self.bitmaps.as_chunks::<BITMAP_LEN>().0.iter().enumerate() {
            let start = self
                .start(vertex)
                .ok_or_else(|| malformed("a start is missing"))?;
            let end = self.start(vertex.saturating_add(1)).unwrap_or(entries);
            let set: usize = bitmap.iter().map(|byte| byte.count_ones() as usize).sum();
            if end > entries || end.checked_sub(start) != Some(set) {
                return Err(malformed(
                    "the bitmap of a vertex does not count its entries",
                ));
            }
            let beyond = (self.blendshapes..MAX_SKIN_BLENDSHAPES).any(|number| {
                bitmap
                    .get(number / 8)
                    .is_some_and(|&byte| byte >> (number % 8) & 1 != 0)
            });
            if beyond {
                return Err(malformed(
                    "a vertex is moved by a blendshape the head lacks",
                ));
            }
        }
        Ok(())
    }

    fn start(&self, vertex: usize) -> Option<usize> {
        u32_at(self.starts, vertex.checked_mul(4)?).and_then(|start| usize::try_from(start).ok())
    }

    /// Returns the number of blendshapes.
    #[must_use]
    pub const fn blendshapes(&self) -> usize {
        self.blendshapes
    }

    /// Returns the number of vertices, those of the geometry file's stream.
    #[must_use]
    pub const fn vertices(&self) -> usize {
        self.starts.len() / 4
    }

    /// Returns blendshape `blendshape` of the head, without the moves under a tenth of a
    /// micrometre; empty for a blendshape no vertex has or one outside the head.
    #[must_use]
    pub fn target(&self, blendshape: usize) -> ExpressionTarget {
        let mut target = ExpressionTarget::default();
        let Some(range) = blendshape
            .checked_mul(RANGE_LEN)
            .and_then(|at| self.ranges.get(at..at.checked_add(RANGE_LEN)?))
        else {
            return target;
        };
        let (Some(scale), Some(bias)) = (vec3_at(range, 0), vec3_at(range, BIAS_AT)) else {
            return target;
        };
        let (byte, bit) = (blendshape / 8, blendshape % 8);
        for (vertex, bitmap) in self.bitmaps.as_chunks::<BITMAP_LEN>().0.iter().enumerate() {
            if bitmap.get(byte).is_none_or(|&bits| bits >> bit & 1 == 0) {
                continue;
            }
            let before: usize = bitmap
                .iter()
                .take(byte)
                .map(|b| b.count_ones() as usize)
                .sum();
            let within = bitmap
                .get(byte)
                .map_or(0, |&bits| (bits & ((1 << bit) - 1)).count_ones());
            let Some(entry) = self
                .start(vertex)
                .and_then(|start| start.checked_add(before.checked_add(within as usize)?))
            else {
                continue;
            };
            let Some(codes) = entry
                .checked_mul(CODE_LEN)
                .and_then(|at| self.codes.get(at..at.checked_add(CODE_LEN)?))
            else {
                continue;
            };
            let delta: [f32; 3] = std::array::from_fn(|axis| {
                let code = codes.get(axis).copied().unwrap_or_default();
                let (scale, bias) = (scale.get(axis), bias.get(axis));
                f32::from(code).mul_add(
                    scale.copied().unwrap_or_default(),
                    bias.copied().unwrap_or_default(),
                )
            });
            if delta.iter().all(|value| value.is_finite())
                && delta.iter().any(|value| value.abs() >= NEGLIGIBLE)
                && let Ok(vertex) = u32::try_from(vertex)
            {
                target.vertices.push(vertex);
                target.deltas.push(delta);
            }
        }
        target
    }
}
