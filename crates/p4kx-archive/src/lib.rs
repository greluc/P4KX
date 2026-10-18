//! Reads Star Citizen `Data.p4k` Archives in two layouts: v1, a ZIP64 container whose central
//! directory carries CIG-specific extra blocks, and v2, fixed 204-byte directory records with a
//! name table behind a trailer ending in `Jiji`. Entries may be stored, deflate or zstd compressed
//! and AES-128-CBC encrypted; their data is streamed through decryption and decompression.

mod archive;
mod central_directory;
mod compression;
mod data_error;
mod decode;
mod decrypt;
mod directory_v2;
mod end_records;
mod entry;
mod entry_reader;
mod error;
mod format_error;
mod layout;
mod source;

pub use archive::Archive;
pub use compression::Compression;
pub use data_error::DataError;
pub use entry::Entry;
pub use entry_reader::EntryReader;
pub use error::ArchiveError;
