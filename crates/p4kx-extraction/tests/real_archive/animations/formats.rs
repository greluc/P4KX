//! A census of the key format codes the controllers of `.caf` files and `.dba` databases use.

use std::collections::BTreeMap;

use p4kx_chunks::{ChunkFile, ivo};

const BLOCK_HEADER_LEN: usize = 12;
const CONTROLLER_LEN: usize = 24;
const SIDE_LEN: usize = 12;
const PREFIX_LEN: usize = 4;

/// How often each rotation, position and time format code occurs.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Formats {
    pub(crate) rotations: BTreeMap<u8, u64>,
    pub(crate) positions: BTreeMap<u8, u64>,
    pub(crate) times: BTreeMap<u8, u64>,
    pub(crate) chunk_versions: BTreeMap<(&'static str, u32), u64>,
}

impl Formats {
    pub(crate) fn merge(mut self, other: Self) -> Self {
        for (mine, theirs) in [
            (&mut self.rotations, other.rotations),
            (&mut self.positions, other.positions),
            (&mut self.times, other.times),
        ] {
            for (code, count) in theirs {
                *mine.entry(code).or_default() += count;
            }
        }
        for (version, count) in other.chunk_versions {
            *self.chunk_versions.entry(version).or_default() += count;
        }
        self
    }

    fn side(&mut self, rotation: bool, controller: &[u8]) {
        let at = if rotation { 0 } else { SIDE_LEN };
        let count = u16_at(controller, at);
        let [times, kind] = u16_at(controller, at + 2).to_le_bytes();
        if count == 0 {
            return;
        }
        let kinds = if rotation {
            &mut self.rotations
        } else {
            &mut self.positions
        };
        *kinds.entry(kind).or_default() += 1;
        *self.times.entry(times).or_default() += 1;
    }

    fn block(&mut self, data: &[u8], at: usize) -> Option<usize> {
        let controllers = usize::from(u16_at(data.get(at..)?, 4));
        let first = at + BLOCK_HEADER_LEN + 4 * controllers;
        let end = first + controllers * CONTROLLER_LEN;
        for entry in data.get(first..end)?.as_chunks::<CONTROLLER_LEN>().0 {
            self.side(true, entry);
            self.side(false, entry);
        }
        Some(end)
    }
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    bytes
        .get(at..at + 2)
        .and_then(|pair| <[u8; 2]>::try_from(pair).ok())
        .map_or(0, u16::from_le_bytes)
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    bytes
        .get(at..at + 4)
        .and_then(|word| <[u8; 4]>::try_from(word).ok())
        .map_or(0, u32::from_le_bytes)
}

/// Counts the format codes of a `.caf` file or a `.dba` database.
pub(crate) fn of<B: AsRef<[u8]>>(file: &ChunkFile<B>) -> Formats {
    let mut formats = Formats::default();
    for (name, chunk_type) in [
        ("AnimInfo", ivo::ANIM_INFO),
        ("CafData", ivo::CAF_DATA),
        ("DbaData", ivo::DBA_DATA),
        ("DbaMetadata", ivo::DBA_METADATA),
    ] {
        for (_, chunk) in file.chunks_of(chunk_type) {
            *formats
                .chunk_versions
                .entry((name, chunk.version()))
                .or_default() += 1;
        }
    }
    if let Some((index, _)) = file.chunks_of(ivo::CAF_DATA).next()
        && let Some(data) = file.chunk_data_to_end(index)
    {
        formats.block(data, 0);
    }
    let database = file
        .chunks_of(ivo::DBA_DATA)
        .next()
        .zip(file.chunks_of(ivo::DBA_METADATA).next());
    if let Some(((data_index, _), (meta_index, _))) = database
        && let Some(data) = file.chunk_data_to_end(data_index)
    {
        let count = file
            .chunk_data(meta_index)
            .map_or(0, |metadata| u32_at(metadata, 0));
        let mut at = PREFIX_LEN;
        for _ in 0..count {
            match formats.block(data, at) {
                Some(end) => at = end,
                None => break,
            }
        }
    }
    formats
}

/// Returns `code: count` for each entry.
pub(crate) fn list(codes: &BTreeMap<u8, u64>) -> String {
    let entries: Vec<String> = codes
        .iter()
        .map(|(code, count)| format!("{code:#04x}: {count}"))
        .collect();
    entries.join(", ")
}

/// Returns `chunk version: count` for each entry.
pub(crate) fn versions(versions: &BTreeMap<(&'static str, u32), u64>) -> String {
    let entries: Vec<String> = versions
        .iter()
        .map(|((chunk, version), count)| format!("{chunk} {version:#x}: {count}"))
        .collect();
    entries.join(", ")
}
