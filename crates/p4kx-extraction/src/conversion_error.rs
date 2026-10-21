use std::io;
use std::path::PathBuf;

use p4kx_archive::ArchiveError;
use p4kx_audio::{AudioError, BankError};
use p4kx_chunks::ChunkError;
use p4kx_gfx::GfxError;
use p4kx_gltf::GltfError;
use p4kx_localization::LocalizationError;
use p4kx_model::{AnimationError, ModelError};
use p4kx_texture::TextureError;
use thiserror::Error;

use crate::UnknownEntries;

/// A failure to convert a texture, a sound or a UI movie.
#[derive(Debug, Error)]
pub enum ConversionError {
    /// The Archive holds no Entry with the requested path.
    #[error(transparent)]
    UnknownEntry(#[from] UnknownEntries),
    /// The header or sound file on disk does not exist.
    #[error("{} does not exist", path.display())]
    NoSuchFile {
        /// The requested file.
        path: PathBuf,
    },
    /// The Archive could not be read.
    #[error(transparent)]
    Archive(#[from] ArchiveError),
    /// A file on disk could not be read.
    #[error("cannot read {}", path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// A sibling part the header needs does not exist.
    #[error("{part} is missing")]
    MissingPart {
        /// The path of the part.
        part: String,
    },
    /// A sibling part is shorter than its mip.
    #[error("{part} is truncated: {actual} of {expected} bytes")]
    TruncatedPart {
        /// The path of the part.
        part: String,
        /// The size of its mip.
        expected: u64,
        /// The bytes it holds.
        actual: u64,
    },
    /// A sibling part is longer than its mip.
    #[error("{part} is {actual} bytes long, but its mip has {expected}")]
    OversizedPart {
        /// The path of the part.
        part: String,
        /// The size of its mip.
        expected: u64,
        /// The bytes it holds.
        actual: u64,
    },
    /// A sibling part could not be read.
    #[error("cannot read {part}")]
    ReadPart {
        /// The path of the part.
        part: String,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// The texture cannot be read or merged.
    #[error("{texture}")]
    Texture {
        /// The path of the header Entry or file.
        texture: String,
        /// What is wrong.
        #[source]
        source: TextureError,
    },
    /// An Entry's data could not be read.
    #[error("cannot read {entry}")]
    ReadEntry {
        /// The Entry path.
        entry: String,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// The sound or soundbank is larger than the limit for reading it into memory.
    #[error("{sound} is larger than {limit} bytes")]
    SoundTooLarge {
        /// The Entry path or file of the sound.
        sound: String,
        /// The limit in bytes.
        limit: u64,
    },
    /// The sound cannot be read or converted.
    #[error("{sound}")]
    Audio {
        /// The Entry path or file of the sound.
        sound: String,
        /// What is wrong.
        #[source]
        source: AudioError,
    },
    /// The soundbank is not valid.
    #[error("{bank}")]
    Soundbank {
        /// The Entry path or file of the soundbank.
        bank: String,
        /// What is wrong, naming the section.
        #[source]
        source: BankError,
    },
    /// The soundbank holds only the prefetched start of a streamed WEM, which cannot be converted.
    #[error(
        "{bank} WEM {id} is only the prefetched start of a streamed WEM; convert the loose WEM {id}.wem"
    )]
    PrefetchedWem {
        /// The Entry path or file of the soundbank.
        bank: String,
        /// The WEM ID.
        id: u32,
    },
    /// The UI movie is larger than the limit for reading it into memory.
    #[error("{movie} is larger than {limit} bytes")]
    UiMovieTooLarge {
        /// The Entry path or file of the movie.
        movie: String,
        /// The limit in bytes.
        limit: u64,
    },
    /// The UI movie is no valid GFx or SWF movie.
    #[error("{movie}")]
    UiMovie {
        /// The Entry path or file of the movie.
        movie: String,
        /// What is wrong.
        #[source]
        source: GfxError,
    },
    /// A definition of the UI movie cannot be decoded or written.
    #[error("{movie} definition {id}")]
    UiAsset {
        /// The Entry path or file of the movie.
        movie: String,
        /// The ID of the definition.
        id: u16,
        /// What is wrong.
        #[source]
        source: GfxError,
    },
    /// The canvas a UI movie is rendered on is empty or too large.
    #[error("cannot render {movie} at this size")]
    UiCanvas {
        /// The Entry path or file of the movie.
        movie: String,
        /// What is wrong.
        #[source]
        source: GfxError,
    },
    /// The `global.ini` that localises the texts of a rendered UI movie cannot be indexed.
    #[error("cannot index the texts {path} of the UI movie")]
    UiTexts {
        /// The Entry path of the `global.ini`.
        path: String,
        /// What is wrong.
        #[source]
        source: LocalizationError,
    },
    /// A bitmap fill of a rendered UI movie names a bitmap the movie does not define.
    #[error("{movie} defines no bitmap {id}")]
    UiBitmapMissing {
        /// The Entry path or file of the movie.
        movie: String,
        /// The ID of the bitmap.
        id: u16,
    },
    /// The bitmaps of a rendered frame of a UI movie hold more pixels than the limit.
    #[error(
        "{movie} bitmap {id} is not decoded: the bitmaps of the frame would hold more than {limit} pixels"
    )]
    UiRenderBitmaps {
        /// The Entry path or file of the movie.
        movie: String,
        /// The ID of the bitmap.
        id: u16,
        /// The limit in pixels.
        limit: u64,
    },
    /// A sub-image cuts a bitmap that is missing or cannot be decoded, or a rectangle outside it.
    #[error(
        "{movie} definition {id} cuts bitmap {image_id}, which is missing, cannot be decoded or is smaller"
    )]
    UiSubImage {
        /// The Entry path or file of the movie.
        movie: String,
        /// The ID of the sub-image.
        id: u16,
        /// The ID of the bitmap it cuts.
        image_id: u16,
    },
    /// The texture an external image of a UI movie refers to does not exist.
    #[error("{movie} definition {id} refers to {file}, which does not exist; tried {}", tried.join(", "))]
    ExternalImageMissing {
        /// The Entry path or file of the movie.
        movie: String,
        /// The ID of the external image.
        id: u16,
        /// The file name the movie gives.
        file: String,
        /// The Entry paths or files looked for.
        tried: Vec<String>,
    },
    /// The file an external image of a UI movie on disk refers to lies outside the folder it must
    /// stay in: the nearest ancestor folder named `Data`, or else the movie's folder.
    #[error("{movie} definition {id} refers to {file}, which lies outside {}; it is not read", root.display())]
    ExternalImageOutside {
        /// The file of the movie.
        movie: String,
        /// The ID of the external image.
        id: u16,
        /// The file name the movie gives.
        file: String,
        /// The folder the file must stay in.
        root: PathBuf,
    },
    /// The chunk file is larger than the limit for reading it into memory.
    #[error("{file} is larger than {limit} bytes")]
    ChunkFileTooLarge {
        /// The Entry path or file of the chunk file.
        file: String,
        /// The limit in bytes.
        limit: u64,
    },
    /// The chunk file is not valid.
    #[error("{file}")]
    ChunkFile {
        /// The Entry path or file of the chunk file.
        file: String,
        /// What is wrong.
        #[source]
        source: ChunkError,
    },
    /// The chunk file has no chunk at the requested index.
    #[error("{file} has no chunk {index}; its {count} chunks are numbered from 0")]
    NoSuchChunk {
        /// The Entry path or file of the chunk file.
        file: String,
        /// The requested index.
        index: usize,
        /// The number of chunks.
        count: usize,
    },
    /// The model or one of its companions is missing, unreadable or corrupt.
    #[error("{file}")]
    Model {
        /// The Entry path or file of the model.
        file: String,
        /// What is wrong.
        #[source]
        source: ModelError,
    },
    /// An animation file is unsupported or corrupt.
    #[error("{file}")]
    Animation {
        /// The Entry path or file of the animation file.
        file: String,
        /// What is wrong.
        #[source]
        source: AnimationError,
    },
    /// A file named as a source of animations is no `.caf` or `.dba` file.
    #[error("{file} is no .caf or .dba file")]
    NotAnimationFile {
        /// The Entry path or file.
        file: String,
    },
    /// No animation has the name that was asked for.
    #[error("no animation is named {}", names.join(", "))]
    UnknownAnimations {
        /// The names no animation has.
        names: Vec<String>,
    },
    /// The model cannot be exported as GLB.
    #[error("{file}")]
    Glb {
        /// The Entry path or file of the model.
        file: String,
        /// What is wrong.
        #[source]
        source: GltfError,
    },
    /// The output file is the header or one of its parts, the sound file, or a file of the model.
    #[error("{} is one of the inputs", path.display())]
    OutputIsInput {
        /// The output file.
        path: PathBuf,
    },
    /// The output file could not be written.
    #[error("cannot write {}", path.display())]
    Write {
        /// The output file.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// The output lies inside a Star Citizen installation; nothing is written there.
    #[error(transparent)]
    Installation(#[from] p4kx_guard::WriteRefused),
}

impl ConversionError {
    /// Names the part of a [`TextureError`] about one part, `<texture>.N` or for an alpha tail
    /// `<texture>.a` its part `<texture>.Na`, or wraps it with the texture's name.
    pub(crate) fn of_texture(texture: &str, error: TextureError) -> Self {
        let part = |index: u32| match texture
            .len()
            .checked_sub(2)
            .and_then(|at| texture.split_at_checked(at))
        {
            Some((base, suffix @ (".a" | ".A"))) => {
                format!("{base}.{index}{}", suffix.trim_start_matches('.'))
            }
            _ => format!("{texture}.{index}"),
        };
        match error {
            TextureError::MissingPart { index } => Self::MissingPart { part: part(index) },
            TextureError::TruncatedPart {
                index,
                expected,
                actual,
            } => Self::TruncatedPart {
                part: part(index),
                expected,
                actual,
            },
            TextureError::OversizedPart {
                index,
                expected,
                actual,
            } => Self::OversizedPart {
                part: part(index),
                expected,
                actual,
            },
            TextureError::ReadPart { index, source } => Self::ReadPart {
                part: part(index),
                source,
            },
            source => Self::Texture {
                texture: texture.to_owned(),
                source,
            },
        }
    }
}
