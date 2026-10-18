//! Shared building blocks of the P4K-Extractor format crates: a bounds-checked little-endian
//! reader over byte slices, the parse error it reports, and the public `CryEngine` Archive key and IV.

mod archive_key;
mod byte_reader;
mod parse_error;

pub use archive_key::{ARCHIVE_IV, ARCHIVE_KEY};
pub use byte_reader::ByteReader;
pub use parse_error::ParseError;
