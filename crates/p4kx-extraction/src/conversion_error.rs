use std::io;
use std::path::PathBuf;

use p4kx_archive::ArchiveError;
use p4kx_texture::TextureError;
use thiserror::Error;

use crate::UnknownEntries;

/// A failure to convert a texture.
#[derive(Debug, Error)]
pub enum ConversionError {
    /// The Archive holds no Entry with the requested path.
    #[error(transparent)]
    UnknownEntry(#[from] UnknownEntries),
    /// The header file on disk does not exist.
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
    /// The output file is the header or one of its parts.
    #[error("{} is an input of the merge", path.display())]
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
}

impl ConversionError {
    /// Names the part of a [`TextureError`] about one part, or wraps it with the texture's name.
    pub(crate) fn of_texture(texture: &str, error: TextureError) -> Self {
        let part = |index: u32| format!("{texture}.{index}");
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
