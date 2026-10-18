use std::io;

use thiserror::Error;

use crate::TextureFormat;

/// A failure to read or merge a DDS texture.
#[derive(Debug, Error)]
pub enum TextureError {
    /// The data does not start with a DDS header.
    #[error("not a DDS texture: the header is missing")]
    NotDds,
    /// The data ends inside the DDS header or before the header Entry's declared size.
    #[error("the DDS header is truncated: {len} bytes, {needed} needed")]
    TruncatedHeader {
        /// Bytes available.
        len: u64,
        /// Bytes needed.
        needed: u64,
    },
    /// The DDS header contradicts itself.
    #[error("the DDS header is invalid: {0}")]
    InvalidHeader(&'static str),
    /// The pixel format's size is unknown, so the mips cannot be told apart.
    #[error("the texture format {0} is not supported")]
    UnsupportedFormat(TextureFormat),
    /// The texture has several images per mip and is split into parts.
    #[error("merging a split texture with {surfaces} images per mip is not supported yet")]
    UnsupportedLayout {
        /// Cubemap faces times array elements.
        surfaces: u32,
    },
    /// The bytes after the header are not a run of the smallest mips.
    #[error("the {tail} bytes after the DDS header are not a whole number of mips")]
    TailMismatch {
        /// Bytes after the header.
        tail: u64,
    },
    /// A sibling part the header needs does not exist.
    #[error("part {index} is missing")]
    MissingPart {
        /// The part's number, the `N` of `.dds.N`.
        index: u32,
    },
    /// A sibling part is shorter than its mip.
    #[error("part {index} is truncated: {actual} of {expected} bytes")]
    TruncatedPart {
        /// The part's number.
        index: u32,
        /// The size of its mip.
        expected: u64,
        /// The bytes it holds.
        actual: u64,
    },
    /// A sibling part is longer than its mip.
    #[error("part {index} is {actual} bytes long, but its mip has {expected}")]
    OversizedPart {
        /// The part's number.
        index: u32,
        /// The size of its mip.
        expected: u64,
        /// The bytes it holds.
        actual: u64,
    },
    /// The header could not be read.
    #[error("cannot read the DDS header")]
    ReadHeader(#[source] io::Error),
    /// A sibling part could not be read.
    #[error("cannot read part {index}")]
    ReadPart {
        /// The part's number.
        index: u32,
        /// The underlying error.
        #[source]
        source: io::Error,
    },
    /// The merged texture could not be written.
    #[error("cannot write the merged texture")]
    Write(#[source] io::Error),
}
