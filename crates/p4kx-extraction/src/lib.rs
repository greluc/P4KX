//! Extraction service of P4K-Extractor.
//!
//! Front-ends depend on this crate instead of the format crates; it provides the Archive types
//! they work with.

pub use p4kx_archive::{Archive, ArchiveError, Compression, Entry};
