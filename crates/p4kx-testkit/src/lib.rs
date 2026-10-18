//! Writers for synthetic test data in the formats P4K-Extractor reads.
//!
//! The writers reproduce the byte layout of real Star Citizen files so that every reader can be
//! tested without game files; [`real_archive_path`] locates a real Archive for the opt-in suite.
//! This crate is a development dependency only.

mod archive_builder;
mod build_manifest;
mod bytes;
mod cryxml;
mod layout_v1;
mod layout_v2;
mod real_archive;
mod split_dds;
mod test_entry;
mod wem;

pub use archive_builder::ArchiveBuilder;
pub use build_manifest::write_build_manifest;
pub use cryxml::{CryXmlByteOrder, CryXmlSignature, TestElement};
pub use real_archive::{MissingRealArchive, REAL_ARCHIVE_VAR, real_archive_path};
pub use split_dds::{SplitDds, TestTextureFormat};
pub use test_entry::TestEntry;
pub use wem::{TestCodec, TestWem, test_signal};
