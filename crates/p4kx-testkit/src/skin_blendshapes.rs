// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::bytes::put_u32;

/// The chunk type of the `BShapesGpu` chunk of a head's `.skinm`.
pub const BSHAPES_GPU_CHUNK: u32 = 0x57A3_8888;

const HEADER_LEN: usize = 32;
const BITMAP_LEN: usize = 64;

/// One entry of a synthetic `BShapesGpu` chunk: a blendshape that moves a vertex, with the byte
/// codes of its move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TestSkinEntry {
    /// The vertex the blendshape moves.
    pub vertex: usize,
    /// The blendshape.
    pub blendshape: usize,
    /// The codes of the move on x, y and z.
    pub codes: [u8; 3],
}

/// A synthetic `BShapesGpu` chunk with the layout of the real ones: a start per vertex, a scale
/// and bias per blendshape, a bitmap per vertex and the codes of the entries.
#[derive(Debug, Clone, PartialEq)]
pub struct TestSkinBlendshapes {
    /// The number of vertices of the stream.
    pub vertices: usize,
    /// The scale and bias of the codes of each blendshape on x, y and z.
    pub ranges: Vec<([f32; 3], [f32; 3])>,
    /// The entries, in any order.
    pub entries: Vec<TestSkinEntry>,
}

impl TestSkinBlendshapes {
    /// Four vertices and three blendshapes with the scale 0.001 and the bias -0.1 on every axis:
    /// blendshape 1 moves vertex 1 by the codes 150, 100, 100 (0.05 m, 0 m, 0 m) and vertex 3 by
    /// 100, 100, 200 (0 m, 0 m, 0.1 m), blendshape 2 moves vertex 1 by 100, 150, 100.
    #[must_use]
    pub fn small() -> Self {
        let entry = |vertex, blendshape, codes| TestSkinEntry {
            vertex,
            blendshape,
            codes,
        };
        Self {
            vertices: 4,
            ranges: vec![([0.001; 3], [-0.1; 3]); 3],
            entries: vec![
                entry(3, 1, [100, 100, 200]),
                entry(1, 2, [100, 150, 100]),
                entry(1, 1, [150, 100, 100]),
            ],
        }
    }

    /// Returns the bytes of the chunk.
    #[must_use]
    pub fn build(&self) -> Vec<u8> {
        let mut sorted = self.entries.clone();
        sorted.sort_by_key(|entry| (entry.vertex, entry.blendshape));
        let mut starts = Vec::new();
        let mut bitmaps = vec![0_u8; self.vertices * BITMAP_LEN];
        let mut codes = Vec::new();
        for vertex in 0..self.vertices {
            put_u32(&mut starts, u32::try_from(codes.len() / 3).unwrap_or(0));
            for entry in sorted.iter().filter(|entry| entry.vertex == vertex) {
                if let Some(byte) = bitmaps.get_mut(vertex * BITMAP_LEN + entry.blendshape / 8) {
                    *byte |= 1 << (entry.blendshape % 8);
                }
                codes.extend_from_slice(&entry.codes);
            }
        }
        let ranges: Vec<u8> = self
            .ranges
            .iter()
            .flat_map(|(scale, bias)| {
                scale
                    .iter()
                    .chain(bias)
                    .flat_map(|value| value.to_le_bytes())
            })
            .collect();
        let starts_at = HEADER_LEN;
        let ranges_at = starts_at + starts.len();
        let bitmaps_at = ranges_at + ranges.len();
        let codes_at = bitmaps_at + bitmaps.len();
        let names_at = codes_at + codes.len();
        let mut out = Vec::new();
        for word in [
            self.ranges.len(),
            self.vertices,
            sorted.len(),
            starts_at,
            ranges_at,
            bitmaps_at,
            codes_at,
            names_at,
        ] {
            put_u32(&mut out, u32::try_from(word).unwrap_or(0));
        }
        out.extend_from_slice(&starts);
        out.extend_from_slice(&ranges);
        out.extend_from_slice(&bitmaps);
        out.extend_from_slice(&codes);
        out
    }
}
