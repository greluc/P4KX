// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Shared building blocks of the P4KX format crates: a bounds-checked little-endian
//! reader over byte slices, the parse error it reports, path matching with the rules of a Filter,
//! and the Archive key type and IV.

mod archive_key;
mod byte_reader;
mod parse_error;
mod path_pattern;

pub use archive_key::{ARCHIVE_IV, ArchiveKey, ArchiveKeyError};
pub use byte_reader::ByteReader;
pub use parse_error::ParseError;
pub use path_pattern::PathPattern;
