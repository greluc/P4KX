// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! How far the key data of a `.dba` database lies from its `DbaData` chunk's computed span.

use p4kx_chunks::{ChunkFile, ivo};

const BLOCK_HEADER_LEN: usize = 12;
const CONTROLLER_LEN: usize = 24;
const PREFIX_LEN: usize = 4;

/// What the controllers of one database say about where their key data starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Reach {
    /// The most bytes any key data starts behind the end of the `DbaData` chunk's span.
    pub(crate) beyond_span: usize,
    /// The number of key data starts behind the end of the file.
    pub(crate) beyond_file: usize,
}

fn u32_at(bytes: &[u8], at: usize) -> Option<usize> {
    let word = bytes.get(at..at + 4)?;
    usize::try_from(u32::from_le_bytes(<[u8; 4]>::try_from(word).ok()?)).ok()
}

fn u16_at(bytes: &[u8], at: usize) -> Option<usize> {
    let word = bytes.get(at..at + 2)?;
    Some(usize::from(u16::from_le_bytes(
        <[u8; 2]>::try_from(word).ok()?,
    )))
}

/// Measures `file`'s controllers, or returns `None` when it is no readable database.
pub(crate) fn of<B: AsRef<[u8]>>(file: &ChunkFile<B>) -> Option<Reach> {
    let (data_index, data_chunk) = file.chunks_of(ivo::DBA_DATA).next()?;
    let (meta_index, _) = file.chunks_of(ivo::DBA_METADATA).next()?;
    let data = file.chunk_data_to_end(data_index)?;
    let count = u32_at(file.chunk_data(meta_index)?, 0)?;
    let span = usize::try_from(data_chunk.size()).ok()?;
    let mut at = PREFIX_LEN;
    let mut reach = Reach {
        beyond_span: 0,
        beyond_file: 0,
    };
    for _ in 0..count {
        let controllers = u16_at(data, at + 4)?;
        let first = at + BLOCK_HEADER_LEN + 4 * controllers;
        for index in 0..controllers {
            let entry = first + index * CONTROLLER_LEN;
            for side in [0, CONTROLLER_LEN / 2] {
                if u16_at(data, entry + side)? == 0 {
                    continue;
                }
                for offset in [
                    u32_at(data, entry + side + 4)?,
                    u32_at(data, entry + side + 8)?,
                ] {
                    let start = entry + offset;
                    reach.beyond_span = reach.beyond_span.max(start.saturating_sub(span));
                    if start > data.len() {
                        reach.beyond_file += 1;
                    }
                }
            }
        }
        at = first + controllers * CONTROLLER_LEN;
    }
    Some(reach)
}
