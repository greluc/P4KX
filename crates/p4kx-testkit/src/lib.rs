//! Writers for synthetic test data in the formats P4K-Extractor reads.
//!
//! The writers reproduce the byte layout of real Star Citizen files so that every reader can be
//! tested without game files; [`real_archive_path`] locates a real Archive for the opt-in suite.
//! This crate is a development dependency only.

mod adpcm;
mod archive_builder;
mod bnk;
mod build_manifest;
mod bytes;
mod cryxml;
mod dataforge_builder;
mod dataforge_instances;
mod dataforge_schema;
mod dataforge_value;
mod entity;
mod entity_sample;
mod entity_schema;
mod layout_v1;
mod layout_v2;
mod real_archive;
mod split_dds;
mod test_entry;
mod vorbis;
mod wem;

pub use archive_builder::ArchiveBuilder;
pub use bnk::{TestBank, TestHircObject};
pub use build_manifest::write_build_manifest;
pub use cryxml::{CryXmlByteOrder, CryXmlSignature, TestElement};
pub use dataforge_builder::DataForgeBuilder;
pub use dataforge_schema::{TestPool, TestRecord, TestStruct, TestType};
pub use dataforge_value::{TestInstance, TestValue};
pub use entity::{TestEntity, TestLoadout, TestLoadoutEntry, TestPort};
pub use entity_sample::{SAMPLE_GUN, SAMPLE_MISSING, SAMPLE_MOUNT, SAMPLE_SHIP, sample_ship};
pub use entity_schema::entity_schema;
pub use real_archive::{
    MissingRealArchive, REAL_ARCHIVE_B_VAR, REAL_ARCHIVE_VAR, real_archive_b_path,
    real_archive_path,
};
pub use split_dds::{SplitDds, TestTextureFormat};
pub use test_entry::TestEntry;
pub use vorbis::{TestCodebook, TestVorbis};
pub use wem::{TestCodec, TestWem, test_signal};
