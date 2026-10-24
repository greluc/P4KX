// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_foundation::ParseError;
use thiserror::Error;

/// A failure to read or write a CHF file or its JSON form.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ChfError {
    /// The file is not exactly as long as every CHF file is.
    #[error("a CHF file is {expected} bytes long, this one is {actual}")]
    WrongSize {
        /// The size of every CHF file.
        expected: usize,
        /// The size of the input.
        actual: usize,
    },
    /// The file does not start with the CHF magic.
    #[error("not a CHF file: the magic is {found:#06x} instead of {expected:#06x}")]
    BadMagic {
        /// The CHF magic.
        expected: u16,
        /// The first two bytes of the input.
        found: u16,
    },
    /// The CRC-32C stored in the header does not match the data behind it.
    #[error("CRC-32C mismatch: the header stores {stored:#010x}, the data gives {computed:#010x}")]
    CrcMismatch {
        /// The CRC-32C the header stores.
        stored: u32,
        /// The CRC-32C of the data behind the header.
        computed: u32,
    },
    /// The compressed body does not fit into the file.
    #[error("the compressed body is {size} bytes, at most {max} fit into a CHF file")]
    FrameTooLarge {
        /// The size of the compressed body.
        size: u64,
        /// The most bytes a CHF file has room for.
        max: usize,
    },
    /// The body is larger than any CHF body this crate reads or writes.
    #[error("the body is {size} bytes, at most {max} are supported")]
    BodyTooLarge {
        /// The size the header states or the body has.
        size: u64,
        /// The largest supported body.
        max: usize,
    },
    /// The compressed body is not a valid zstd frame.
    #[error("the compressed body is corrupt: {reason}")]
    CorruptFrame {
        /// What the zstd decoder reported.
        reason: String,
    },
    /// The decompressed body is not as long as the header states.
    #[error("the header states a body of {stored} bytes, the frame holds {actual}")]
    BodySizeMismatch {
        /// The size the header states.
        stored: u32,
        /// The size of the decompressed body.
        actual: usize,
    },
    /// A version field holds a version this crate does not know.
    #[error("unknown {field} version {version}, supported are {supported:?}")]
    UnknownVersion {
        /// Which version field it is.
        field: &'static str,
        /// The version found.
        version: u32,
        /// The versions this crate reads.
        supported: &'static [u32],
    },
    /// A field holds a value that the layout does not allow.
    #[error("unexpected {what} at body offset {offset}: expected {expected}, found {found}")]
    UnexpectedValue {
        /// Offset of the field in the decompressed body.
        offset: u64,
        /// What the field is.
        what: &'static str,
        /// The value the layout requires.
        expected: u128,
        /// The value found.
        found: u128,
    },
    /// The item port tree is nested deeper than this crate follows.
    #[error("the item port tree is nested deeper than {max} levels")]
    TooDeep {
        /// The deepest supported nesting.
        max: usize,
    },
    /// The body ends before a value could be read.
    #[error("the body is truncated: {0}")]
    Truncated(#[from] ParseError),

    /// The Character holds values that the CHF layout cannot store.
    #[error("the character cannot be written as CHF: {reason}")]
    Unwritable {
        /// What does not fit the layout.
        reason: String,
    },
    /// The JSON form is malformed or holds values a CHF file cannot store.
    #[error("invalid CHF JSON: {0}")]
    Json(#[from] serde_json::Error),
}
