//! Writers for synthetic test data in the formats P4K-Extractor reads.
//!
//! The writers reproduce the byte layout of real Star Citizen files so that every reader can be
//! tested without game files; [`real_archive_path`] locates a real Archive for the opt-in suite.
//! This crate is a development dependency only.

mod adpcm;
mod archive_builder;
mod audio;
mod bnk;
mod build_manifest;
mod bytes;
mod chf;
mod chunk_file;
mod container;
mod cryxml;
mod dataforge_builder;
mod dataforge_instances;
mod dataforge_schema;
mod dataforge_value;
mod entity;
mod entity_sample;
mod entity_schema;
mod gfx;
mod gfx_shape;
mod gfx_tags;
mod glb;
mod glb_container;
mod global_ini;
mod layout_v1;
mod layout_v2;
mod model;
mod model_cube;
mod model_nmc;
mod model_skin;
mod opus;
mod opus_encode;
mod real_archive;
mod split_dds;
mod test_entry;
mod test_jpeg;
mod vorbis;
mod wem;

pub use archive_builder::ArchiveBuilder;
pub use audio::{TestAtl, audio_schema, wwise_short_id};
pub use bnk::{TestBank, TestHircObject};
pub use build_manifest::write_build_manifest;
pub use chf::{TestChf, TestItemPort, TestMaterial, TestSubMaterial};
pub use chunk_file::TestChunkFile;
pub use container::TestContainer;
pub use cryxml::{CryXmlByteOrder, CryXmlSignature, TestElement};
pub use dataforge_builder::DataForgeBuilder;
pub use dataforge_schema::{TestPool, TestRecord, TestStruct, TestType};
pub use dataforge_value::{TestInstance, TestValue};
pub use entity::{TestEntity, TestLoadout, TestLoadoutEntry, TestPort};
pub use entity_sample::{SAMPLE_GUN, SAMPLE_MISSING, SAMPLE_MOUNT, SAMPLE_SHIP, sample_ship};
pub use entity_schema::entity_schema;
pub use gfx::{TestMovie, TestSignature, tag_bytes, zlib};
pub use gfx_shape::{TestFill, TestShape, TestShapeRecord, morph_shape_body};
pub use gfx_tags::{
    TestGlyph, color_mapped_body, exporter_info_body, exports_body, external_image_body,
    font_name_body, font3_body, jpeg_body, lossless_body, place_object2_body, sprite_body,
    sub_image_body,
};
pub use glb::{CheckedGlb, check_glb};
pub use global_ini::GlobalIniBuilder;
pub use gltf;
pub use model::{
    CUBE_CAPS_MATERIAL, CUBE_MATERIAL_LIBRARY, CUBE_MESH_OFFSET, CUBE_SIDES_MATERIAL, TestModel,
    TestNode, TestNormals, TestPositions, TestSubmesh, TestVertex,
};
pub use opus::{TestOpus, TestOpusError, TestOpusFraming, TestOpusPackets, wwise_opus_wem};
pub use real_archive::{
    MissingRealArchive, REAL_ARCHIVE_B_VAR, REAL_ARCHIVE_VAR, real_archive_b_path,
    real_archive_path,
};
pub use split_dds::{SplitDds, TestTextureFormat};
pub use test_entry::TestEntry;
pub use test_jpeg::{gray_jpeg, progressive_gray_jpeg, sampled_jpeg};
pub use vorbis::{TestCodebook, TestVorbis};
pub use wem::{TestCodec, TestWem, test_signal};
