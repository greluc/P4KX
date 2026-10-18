use p4kx_foundation::ParseError;
use thiserror::Error;

use crate::Container;

/// A failure to read a chunk file.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChunkError {
    /// The file starts with neither `#ivo` nor `CrCh`.
    #[error("not a chunk file: the signature is \"{}\", not \"#ivo\" or \"CrCh\"", signature.escape_ascii())]
    UnknownSignature {
        /// The first four bytes.
        signature: [u8; 4],
    },
    /// The container has a version this crate does not read.
    #[error("{container} version {version:#x} is not supported, only {:#x}", container.version())]
    UnsupportedVersion {
        /// The container form.
        container: Container,
        /// The version found.
        version: u32,
    },
    /// The chunk table reaches past the end of the file.
    #[error("the chunk table of {count} chunks at offset {offset} does not fit in {len} bytes")]
    TableOutOfBounds {
        /// Offset of the table.
        offset: u64,
        /// Number of chunks it declares.
        count: u32,
        /// Length of the file.
        len: u64,
    },
    /// A chunk's data reaches past the end of the file.
    #[error("chunk {index} at offset {offset} with {size} bytes does not fit in {len} bytes")]
    ChunkOutOfBounds {
        /// Position of the chunk in the table.
        index: usize,
        /// Offset of its data.
        offset: u64,
        /// Size it declares; zero in `#ivo`, which declares none.
        size: u64,
        /// Length of the file.
        len: u64,
    },
    /// The file ends inside its header.
    #[error("the chunk file header is truncated")]
    Truncated(#[from] ParseError),
}
