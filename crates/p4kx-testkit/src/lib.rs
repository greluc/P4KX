//! Writers for synthetic test data in the formats P4K-Extractor reads.
//!
//! The writers reproduce the byte layout of real Star Citizen files so that every reader can be
//! tested without game files. This crate is a development dependency only.

mod archive_builder;

pub use archive_builder::{ArchiveBuilder, TestEntry};
