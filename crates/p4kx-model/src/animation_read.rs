// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_chunks::{ChunkFile, Container, ivo};
use p4kx_foundation::ByteReader;

use crate::animation_keys::{bytes_at, read_rotations, read_times, read_translations};
use crate::reader::to_usize;
use crate::{Animation, AnimationError, CorruptAnimation, Keys, Track};

/// The most keys one animation may declare.
pub const MAX_ANIMATION_KEYS: u64 = 1 << 22;
/// The most keys all animations of one database may declare together.
pub const MAX_DATABASE_KEYS: u64 = 1 << 24;

const ANIM_INFO_VERSION: u32 = 0x901;
const CAF_DATA_VERSION: u32 = 0x900;
pub(crate) const RECORD_LEN: usize = 48;
const BLOCK_HEADER_LEN: usize = 12;
const CONTROLLER_LEN: usize = 24;
const SIDE_LEN: usize = 12;
const ADDITIVE: u32 = 1;
const CAF_SIGNATURE: [u8; 4] = *b"#caf";

#[derive(Debug, Clone, Copy)]
pub(crate) struct Record {
    pub(crate) additive: bool,
    pub(crate) frames_per_second: u16,
    pub(crate) end_frame: u32,
}

pub(crate) fn read_record(bytes: &[u8]) -> Result<Record, CorruptAnimation> {
    let mut reader = ByteReader::new(bytes);
    let flags = reader.read_u32()?;
    let frames_per_second = reader.read_u16()?;
    reader.skip(2 + 2 + 2 + 4)?;
    let end_frame = reader.read_u32()?;
    if frames_per_second == 0 {
        return Err(CorruptAnimation::FrameRate);
    }
    Ok(Record {
        additive: flags & ADDITIVE != 0,
        frames_per_second,
        end_frame,
    })
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Block {
    pub(crate) at: usize,
    pub(crate) count: usize,
    pub(crate) end: usize,
}

pub(crate) fn read_block(
    data: &[u8],
    at: usize,
    signature: [u8; 4],
) -> Result<Block, CorruptAnimation> {
    let head = bytes_at(data, at, BLOCK_HEADER_LEN)?;
    let mut reader = ByteReader::new(head);
    let found = <[u8; 4]>::try_from(reader.read_bytes(4)?).unwrap_or_default();
    if found != signature {
        return Err(CorruptAnimation::Signature {
            expected: signature,
            found,
        });
    }
    let count = usize::from(reader.read_u16()?);
    let tables = count * (4 + CONTROLLER_LEN);
    bytes_at(data, at + BLOCK_HEADER_LEN, tables)?;
    Ok(Block {
        at,
        count,
        end: at + BLOCK_HEADER_LEN + tables,
    })
}

#[derive(Debug, Clone, Copy)]
struct Side {
    count: usize,
    format: u8,
    times: u8,
    time_at: usize,
    data_at: usize,
}

impl Side {
    fn read(bytes: &[u8], entry_at: usize) -> Result<Self, CorruptAnimation> {
        let mut reader = ByteReader::new(bytes);
        let count = usize::from(reader.read_u16()?);
        let format = reader.read_u16()?;
        let time = to_usize(reader.read_u32()?);
        let data = to_usize(reader.read_u32()?);
        let [times, kind] = format.to_le_bytes();
        let absolute = |offset: usize| entry_at.saturating_add(offset);
        Ok(Self {
            count,
            format: kind,
            times,
            time_at: absolute(time),
            data_at: absolute(data),
        })
    }
}

pub(crate) fn declared_keys(data: &[u8], block: &Block) -> Result<u64, CorruptAnimation> {
    let controllers_at = block.at + BLOCK_HEADER_LEN + 4 * block.count;
    let controllers = bytes_at(data, controllers_at, block.count * CONTROLLER_LEN)?;
    Ok(controllers
        .as_chunks::<CONTROLLER_LEN>()
        .0
        .iter()
        .map(|entry| {
            let count = |at: usize| {
                entry
                    .get(at..at + 2)
                    .and_then(|pair| <[u8; 2]>::try_from(pair).ok())
                    .map_or(0, u16::from_le_bytes)
            };
            u64::from(count(0)) + u64::from(count(SIDE_LEN))
        })
        .sum())
}

pub(crate) fn decode_tracks(data: &[u8], block: &Block) -> Result<Vec<Track>, CorruptAnimation> {
    let hashes = bytes_at(data, block.at + BLOCK_HEADER_LEN, 4 * block.count)?;
    let controllers_at = block.at + BLOCK_HEADER_LEN + 4 * block.count;
    let mut tracks = Vec::with_capacity(block.count);
    for (index, hash) in hashes.as_chunks::<4>().0.iter().enumerate() {
        let controller_id = u32::from_le_bytes(*hash);
        let entry_at = controllers_at + index * CONTROLLER_LEN;
        let entry = bytes_at(data, entry_at, CONTROLLER_LEN)?;
        let (rotation, translation) = entry.split_at(SIDE_LEN);
        let rotation = Side::read(rotation, entry_at)?;
        let translation = Side::read(translation, entry_at)?;
        let rotations = (rotation.count > 0)
            .then(|| {
                Ok::<_, CorruptAnimation>(Keys {
                    frames: read_times(
                        data,
                        rotation.time_at,
                        rotation.times,
                        rotation.count,
                        controller_id,
                    )?,
                    values: read_rotations(
                        data,
                        rotation.data_at,
                        rotation.format,
                        rotation.count,
                        controller_id,
                    )?,
                })
            })
            .transpose()?;
        let translations = (translation.count > 0)
            .then(|| {
                Ok::<_, CorruptAnimation>(Keys {
                    frames: read_times(
                        data,
                        translation.time_at,
                        translation.times,
                        translation.count,
                        controller_id,
                    )?,
                    values: read_translations(
                        data,
                        translation.data_at,
                        translation.format,
                        translation.count,
                    )?,
                })
            })
            .transpose()?;
        if rotations.is_some() || translations.is_some() {
            tracks.push(Track {
                controller_id,
                rotations,
                translations,
            });
        }
    }
    Ok(tracks)
}

pub(crate) const fn check_budget(keys: u64, limit: u64) -> Result<(), CorruptAnimation> {
    if keys > limit {
        return Err(CorruptAnimation::TooManyKeys { keys, limit });
    }
    Ok(())
}

pub(crate) const fn build(
    name: String,
    path: String,
    record: Record,
    tracks: Vec<Track>,
) -> Animation {
    Animation {
        name,
        path,
        frames_per_second: record.frames_per_second,
        end_frame: record.end_frame,
        additive: record.additive,
        tracks,
    }
}

pub(crate) fn stem(path: &str) -> &str {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file.rsplit_once('.').map_or(file, |(stem, _)| stem)
}

pub(crate) fn corrupt(file: &str, chunk: &'static str, source: CorruptAnimation) -> AnimationError {
    AnimationError::Corrupt {
        file: file.to_owned(),
        chunk,
        source,
    }
}

/// Returns the data of the first chunk of `chunk_type` from its start to the end of the file.
pub(crate) fn chunk_bytes<'f, B: AsRef<[u8]>>(
    file_name: &str,
    file: &'f ChunkFile<B>,
    chunk_type: p4kx_chunks::ChunkType,
    version: u32,
    name: &'static str,
) -> Result<&'f [u8], AnimationError> {
    if file.container() != Container::Ivo {
        return Err(AnimationError::UnsupportedContainer {
            file: file_name.to_owned(),
            container: file.container(),
        });
    }
    let (index, chunk) =
        file.chunks_of(chunk_type)
            .next()
            .ok_or_else(|| AnimationError::MissingChunk {
                file: file_name.to_owned(),
                chunk: name,
            })?;
    if chunk.version() != version {
        return Err(AnimationError::UnsupportedVersion {
            file: file_name.to_owned(),
            chunk: name,
            version: chunk.version(),
        });
    }
    file.chunk_data_to_end(index)
        .ok_or_else(|| AnimationError::MissingChunk {
            file: file_name.to_owned(),
            chunk: name,
        })
}

impl Animation {
    /// Reads the animation of the `.caf` file `file` named `file_name`: its `AnimInfo` chunk
    /// and its `CafData` chunk with one block of controllers. The animation is named after the
    /// file without folder and extension.
    ///
    /// # Errors
    ///
    /// Returns [`AnimationError::UnsupportedContainer`] for a `CrCh` file,
    /// [`AnimationError::MissingChunk`] and [`AnimationError::UnsupportedVersion`] when a chunk is
    /// absent or has a layout this reader does not know, and [`AnimationError::Corrupt`] when
    /// the data is truncated, malformed or in a key format this reader does not know.
    pub fn read_caf<B: AsRef<[u8]>>(
        file_name: &str,
        file: &ChunkFile<B>,
    ) -> Result<Self, AnimationError> {
        let info = chunk_bytes(
            file_name,
            file,
            ivo::ANIM_INFO,
            ANIM_INFO_VERSION,
            "AnimInfo",
        )?;
        let record = read_record(info.get(..RECORD_LEN).unwrap_or(info))
            .map_err(|source| corrupt(file_name, "AnimInfo", source))?;
        let data = chunk_bytes(file_name, file, ivo::CAF_DATA, CAF_DATA_VERSION, "CafData")?;
        let tracks = read_block(data, 0, CAF_SIGNATURE)
            .and_then(|block| {
                check_budget(declared_keys(data, &block)?, MAX_ANIMATION_KEYS)?;
                decode_tracks(data, &block)
            })
            .map_err(|source| corrupt(file_name, "CafData", source))?;
        Ok(build(
            stem(file_name).to_owned(),
            file_name.to_owned(),
            record,
            tracks,
        ))
    }
}
