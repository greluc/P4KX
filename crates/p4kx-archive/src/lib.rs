//! Reads Star Citizen `Data.p4k` Archives: a ZIP64 container whose central directory carries
//! CIG-specific extra blocks.

mod archive;
mod central_directory;
mod compression;
mod end_records;
mod entry;
mod error;
mod format_error;
mod source;

pub use archive::Archive;
pub use compression::Compression;
pub use entry::Entry;
pub use error::ArchiveError;
