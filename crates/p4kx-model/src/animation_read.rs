use p4kx_chunks::{ChunkFile, Container, ivo};
use p4kx_foundation::ByteReader;
use rayon::prelude::*;

use crate::animation_keys::{bytes_at, read_rotations, read_times, read_translations};
use crate::reader::to_usize;
use crate::{Animation, AnimationError, CorruptAnimation, Keys, Track};

/// The most keys one animation may declare.
pub const MAX_ANIMATION_KEYS: u64 = 1 << 22;
/// The most keys all animations of one database may declare together.
pub const MAX_DATABASE_KEYS: u64 = 1 << 24;

const ANIM_INFO_VERSION: u32 = 0x901;
const CAF_DATA_VERSION: u32 = 0x900;
const DBA_DATA_VERSION: u32 = 0x900;
const DBA_METADATA_VERSION: u32 = 0x902;
const RECORD_LEN: usize = 48;
const BLOCK_HEADER_LEN: usize = 12;
const CONTROLLER_LEN: usize = 24;
const SIDE_LEN: usize = 12;
const PREFIX_LEN: usize = 4;
const METADATA_HEADER_LEN: usize = 8;
const ADDITIVE: u32 = 1;
const CAF_SIGNATURE: [u8; 4] = *b"#caf";
const DBA_SIGNATURE: [u8; 4] = *b"#dba";

/// An animation of a `.dba` database that could not be read, with the reason; the other
/// animations of the database are unaffected.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SkippedAnimation {
    /// The animation's name.
    pub name: String,
    /// The path the database lists it under.
    pub path: String,
    /// Why it could not be read.
    pub reason: CorruptAnimation,
}

/// The animations of a `.dba` animation database.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Dba {
    /// The animations that were read, in the database's order.
    pub animations: Vec<Animation>,
    /// The animations that could not be read.
    pub skipped: Vec<SkippedAnimation>,
}

#[derive(Debug, Clone, Copy)]
struct Record {
    additive: bool,
    frames_per_second: u16,
    end_frame: u32,
}

fn read_record(bytes: &[u8]) -> Result<Record, CorruptAnimation> {
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
struct Block {
    at: usize,
    count: usize,
    end: usize,
}

fn read_block(data: &[u8], at: usize, signature: [u8; 4]) -> Result<Block, CorruptAnimation> {
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

fn declared_keys(data: &[u8], block: &Block) -> Result<u64, CorruptAnimation> {
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

fn decode_tracks(data: &[u8], block: &Block) -> Result<Vec<Track>, CorruptAnimation> {
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

const fn check_budget(keys: u64, limit: u64) -> Result<(), CorruptAnimation> {
    if keys > limit {
        return Err(CorruptAnimation::TooManyKeys { keys, limit });
    }
    Ok(())
}

const fn build(name: String, path: String, record: Record, tracks: Vec<Track>) -> Animation {
    Animation {
        name,
        path,
        frames_per_second: record.frames_per_second,
        end_frame: record.end_frame,
        additive: record.additive,
        tracks,
    }
}

fn stem(path: &str) -> &str {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file.rsplit_once('.').map_or(file, |(stem, _)| stem)
}

fn corrupt(file: &str, chunk: &'static str, source: CorruptAnimation) -> AnimationError {
    AnimationError::Corrupt {
        file: file.to_owned(),
        chunk,
        source,
    }
}

/// Returns the data of the first chunk of `chunk_type` from its start to the end of the file.
fn chunk_bytes<'f, B: AsRef<[u8]>>(
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

impl Dba {
    /// Reads the animations of the `.dba` file `file` named `file_name`: the blocks of
    /// controllers of its `DbaData` chunk and, from its `DbaMetadata` chunk, the name, frame rate and
    /// end frame of each.
    ///
    /// An animation that cannot be read is left out of [`Self::animations`] and listed in
    /// [`Self::skipped`].
    ///
    /// # Errors
    ///
    /// Returns [`AnimationError::UnsupportedContainer`] for a `CrCh` file,
    /// [`AnimationError::MissingChunk`] and [`AnimationError::UnsupportedVersion`] when a chunk is
    /// absent or has a layout this reader does not know, and [`AnimationError::Corrupt`] when
    /// the metadata or a block header is truncated or malformed or the database declares more
    /// keys than [`MAX_DATABASE_KEYS`].
    pub fn read<B: AsRef<[u8]>>(
        file_name: &str,
        file: &ChunkFile<B>,
    ) -> Result<Self, AnimationError> {
        let metadata = chunk_bytes(
            file_name,
            file,
            ivo::DBA_METADATA,
            DBA_METADATA_VERSION,
            "DbaMetadata",
        )?;
        let data = chunk_bytes(file_name, file, ivo::DBA_DATA, DBA_DATA_VERSION, "DbaData")?;
        read_database(metadata, data).map_err(|(chunk, source)| corrupt(file_name, chunk, source))
    }
}

fn read_database(metadata: &[u8], data: &[u8]) -> Result<Dba, (&'static str, CorruptAnimation)> {
    let in_metadata = |source| ("DbaMetadata", source);
    let in_data = |source| ("DbaData", source);
    let mut reader = ByteReader::new(metadata);
    let count = to_usize(
        reader
            .read_u32()
            .map_err(|error| in_metadata(error.into()))?,
    );
    reader
        .skip(METADATA_HEADER_LEN - 4)
        .map_err(|error| in_metadata(error.into()))?;
    let records = reader
        .read_bytes(count.saturating_mul(RECORD_LEN))
        .map_err(|error| in_metadata(error.into()))?;
    let paths = names(
        reader
            .read_bytes(reader.remaining())
            .map_err(|error| in_metadata(error.into()))?,
        count,
    )
    .map_err(in_metadata)?;

    let mut blocks = Vec::with_capacity(count);
    let mut position = PREFIX_LEN;
    let mut keys = 0_u64;
    for _ in 0..count {
        let block = read_block(data, position, DBA_SIGNATURE).map_err(in_data)?;
        keys = keys.saturating_add(declared_keys(data, &block).map_err(in_data)?);
        check_budget(keys, MAX_DATABASE_KEYS).map_err(in_data)?;
        position = block.end;
        blocks.push(block);
    }
    let decoded: Vec<_> = blocks
        .par_iter()
        .zip(records.par_chunks_exact(RECORD_LEN))
        .map(|(block, record)| {
            let record = read_record(record)?;
            check_budget(declared_keys(data, block)?, MAX_ANIMATION_KEYS)?;
            Ok::<_, CorruptAnimation>((record, decode_tracks(data, block)?))
        })
        .collect();
    let mut dba = Dba {
        animations: Vec::with_capacity(count),
        skipped: Vec::new(),
    };
    for (path, result) in paths.into_iter().zip(decoded) {
        let name = stem(&path).to_owned();
        match result {
            Ok((record, tracks)) => dba.animations.push(build(name, path, record, tracks)),
            Err(reason) => dba.skipped.push(SkippedAnimation { name, path, reason }),
        }
    }
    Ok(dba)
}

fn names(table: &[u8], count: usize) -> Result<Vec<String>, CorruptAnimation> {
    let mut parts = table.split_inclusive(|&byte| byte == 0);
    (0..count)
        .map(|_| {
            parts
                .next()
                .and_then(<[u8]>::split_last)
                .filter(|&(&last, _)| last == 0)
                .map(|(_, name)| String::from_utf8_lossy(name).into_owned())
                .ok_or(CorruptAnimation::Names { animations: count })
        })
        .collect()
}
