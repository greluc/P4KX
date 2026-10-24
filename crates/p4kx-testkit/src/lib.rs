// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Writers for synthetic test data in the formats P4KX reads.
//!
//! The writers reproduce the byte layout of real Star Citizen files so that every reader can be
//! tested without game files; [`real_archive_path`] locates a real Archive for the opt-in suite.
//! This crate is a development dependency only.

mod adpcm;
mod animation;
mod animation_dba;
mod animation_keys;
mod archive_builder;
mod audio;
mod bnk;
mod build_manifest;
mod bytes;
mod chf;
mod chunk_file;
mod cli;
mod container;
mod cryxml;
mod dataforge_builder;
mod dataforge_instances;
mod dataforge_schema;
mod dataforge_strings;
mod dataforge_value;
mod dna;
mod entity;
mod entity_armour;
mod entity_armour_loadout;
mod entity_assembly;
mod entity_cargo;
mod entity_dressed;
mod entity_head;
mod entity_interior_sample;
mod entity_layer_chain;
mod entity_layered;
mod entity_livery;
mod entity_loadout;
mod entity_port;
mod entity_sample;
mod entity_schema;
mod entity_vehicle;
mod frames;
mod gfx;
mod gfx_import;
mod gfx_place;
mod gfx_shape;
mod gfx_tags;
mod gfx_text;
mod glb;
mod glb_animation;
mod glb_container;
mod glb_images;
mod glb_lights;
mod glb_morph;
mod glb_skin;
mod global_ini;
mod hirc_node;
mod interior_lit;
mod interior_sample;
mod layout_v1;
mod layout_v2;
mod mesh_streams;
mod model;
mod model_crch;
mod model_cube;
mod model_nmc;
mod model_rig;
mod model_skeleton;
mod model_skin;
mod mtl;
mod opus;
mod opus_encode;
mod real_archive;
mod rig_logic;
mod screens;
mod screens_canvas;
mod screens_geometry;
mod screens_sample;
mod screens_style;
mod screens_text;
mod screens_text_schema;
mod screens_value;
mod screens_widget;
mod soc;
mod soc_entities;
mod split_dds;
mod test_entry;
mod test_jpeg;
mod test_key;
mod vorbis;
mod wem;

pub use animation::{TestAnimation, TestTrack};
pub use animation_dba::TestDba;
pub use animation_keys::{TestPositionFormat, TestRotationFormat, TestTimes};
pub use archive_builder::ArchiveBuilder;
pub use audio::{TestAtl, audio_schema, wwise_short_id};
pub use bnk::{TestBank, TestHircObject};
pub use build_manifest::write_build_manifest;
pub use chf::{TestChf, TestItemPort, TestMaterial, TestSubMaterial};
pub use chunk_file::TestChunkFile;
pub use cli::p4kx;
pub use container::TestContainer;
pub use cryxml::{CryXmlByteOrder, CryXmlSignature, TestElement};
pub use dataforge_builder::DataForgeBuilder;
pub use dataforge_schema::{TestPool, TestRecord, TestStruct, TestType};
pub use dataforge_value::{TestInstance, TestValue};
pub use dna::{TestCell, TestControl, TestDeltaRecord, TestDna};
pub use entity::TestEntity;
pub use entity_armour::{armour_entries, armour_female_skeleton, armour_male_skeleton};
pub use entity_armour_loadout::{ARMOUR_XML_LOADOUT, armour_xml_loadout};
pub use entity_assembly::{ASSEMBLY_GUN_PAINT, assembly_entries, hologram_entries};
pub use entity_cargo::{TestCapacity, closed_container_record, open_container_record};
pub use entity_dressed::{
    DRESSED_BROWS, DRESSED_EYES, DRESSED_EYES_MATERIAL, DRESSED_GONE_HAIR, DRESSED_HAIR,
    DRESSED_HAIR_MATERIAL, DRESSED_HAIR_OPACITY, DRESSED_HEAD_DIFFUSE, DRESSED_HEAD_MATERIAL,
    DRESSED_HEAD_MATERIAL_FILE, DRESSED_UNKNOWN_MATERIAL, dressed_entries,
};
pub use entity_head::{
    FEMALE_RIG_HASH, HEAD_REGIONS, HEAD_VERTICES, MALE_HEAD_FOLDER, MALE_RIG, MALE_RIG_HASH,
    head_entries, head_rig,
};
pub use entity_interior_sample::{
    ENTITY_INTERIOR_CLASS, ENTITY_INTERIOR_FILE, ENTITY_LIT_ROOM_CLASS, entity_interior_entries,
    entity_lit_room_entries,
};
pub use entity_layer_chain::{LAYER_CHAIN_LENGTH, layer_chain_entries};
pub use entity_layered::{LAYERED_MISSING_TEXTURE, LAYERED_TINTS, layered_entries};
pub use entity_livery::{
    LIVERY_DEFAULT_PALETTE, LIVERY_DEFAULT_TINTS, LIVERY_GUN_TINTS, LIVERY_SHIP_TINTS,
    livery_entries,
};
pub use entity_loadout::{TestLoadout, TestLoadoutEntry};
pub use entity_port::TestPort;
pub use entity_sample::{SAMPLE_GUN, SAMPLE_MISSING, SAMPLE_MOUNT, SAMPLE_SHIP, sample_ship};
pub use entity_schema::{entity_schema, tint_palette_record};
pub use entity_vehicle::landing_system_record;
pub use frames::assert_steady_frames;
pub use gfx::{TestMovie, TestSignature, tag_bytes, zlib};
pub use gfx_import::{
    import_assets_body, remove_object_body, scaling_grid_body, symbol_class_body,
};
pub use gfx_place::{TestPlace, background_body, remove_object2_body};
pub use gfx_shape::{TestFill, TestGradient, TestShape, TestShapeRecord, morph_shape_body};
pub use gfx_tags::{
    TestGlyph, color_mapped_body, exporter_info_body, exports_body, external_image_body,
    font_name_body, font3_body, font3_body_styled, jpeg_body, lossless_body, place_object2_body,
    sprite_body, sub_image_body,
};
pub use gfx_text::{TestEditText, TestTextRecord, define_text_body};
pub use glb::{CheckedGlb, check_glb};
pub use glb_images::DecodedImage;
pub use global_ini::GlobalIniBuilder;
pub use gltf;
pub use hirc_node::{TestContainerKind, TestNodeBase};
pub use interior_lit::{LIT_ROOM, lit_room_entries};
pub use interior_sample::{
    INTERIOR_LAMP, INTERIOR_ROOT, INTERIOR_ROOT_NAME, interior_entries, lit_interior_entries,
};
pub use mesh_streams::{MeshStream, MeshStreams, mesh_streams};
pub use model::{
    CUBE_CAPS_MATERIAL, CUBE_MATERIAL_LIBRARY, CUBE_MESH_OFFSET, CUBE_SIDES_MATERIAL, TestModel,
    TestNode, TestNormals, TestPositions, TestSubmesh, TestVertex,
};
pub use model_crch::{CRCH_CUBE_ROOT_OFFSET, TestCrchModel};
pub use model_skeleton::{TestBone, TestSkeleton};
pub use mtl::{TestMtl, TestMtlMaterial};
pub use opus::{TestOpus, TestOpusError, TestOpusFraming, TestOpusPackets, wwise_opus_wem};
pub use real_archive::{
    MissingRealArchive, MissingRealKey, REAL_ARCHIVE_B_VAR, REAL_ARCHIVE_KEY_FILE_VAR,
    REAL_ARCHIVE_KEY_VAR, REAL_ARCHIVE_VAR, real_archive_b_path, real_archive_key,
    real_archive_pair, real_archive_path,
};
pub use rig_logic::{RIG_LOGIC_CHUNK, TestRigLogic};
pub use screens::{SCREEN_COLOUR_SLOTS, screens_schema};
pub use screens_canvas::{TestCanvas, movie_component, screen_component, tag_record};
pub use screens_geometry::clone_modifiers;
pub use screens_sample::{SCREEN_MFD, SCREEN_PANEL, SCREEN_SHIP, SCREEN_STYLE, screens_sample};
pub use screens_style::{TestCondition, TestModifier, TestStyle, TestStyleEntry};
pub use screens_text::{
    SCREEN_TEXT_CANVAS, SCREEN_TEXT_STYLE, screens_font_movie, screens_text_sample,
    text_localisation,
};
pub use screens_widget::{TestColour, TestDim, TestWidget};
pub use soc::{SOC_IDENTITY, TestObjectContainer, TestSoc};
pub use soc_entities::{TestLight, TestLightState, TestSocEntity};
pub use split_dds::{SplitDds, TestTextureFormat};
pub use test_entry::TestEntry;
pub use test_jpeg::{gray_jpeg, progressive_gray_jpeg, sampled_jpeg};
pub use test_key::{other_test_key, test_key};
pub use vorbis::{TestCodebook, TestVorbis};
pub use wem::{TestCodec, TestWem, test_signal};
