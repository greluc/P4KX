//! Shared building blocks of the P4K-Extractor format crates: a bounds-checked little-endian
//! reader over byte slices and the parse error it reports.

mod byte_reader;
mod parse_error;

pub use byte_reader::ByteReader;
pub use parse_error::ParseError;
