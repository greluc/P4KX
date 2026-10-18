//! Writers for synthetic test data in the formats P4K-Extractor reads.
//!
//! The writers reproduce the byte layout of real Star Citizen files so that every reader can be
//! tested without game files. This crate is a development dependency only.

mod archive_builder;
mod bytes;
mod layout_v1;
mod layout_v2;
mod test_entry;

pub use archive_builder::ArchiveBuilder;
pub use test_entry::TestEntry;
