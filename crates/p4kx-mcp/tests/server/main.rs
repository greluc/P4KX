// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx-mcp` end to end: the server runs as a child process and is spoken to over stdio, with a
//! synthetic Library folder and Archive.

mod archive_key;
mod archives;
mod channel_notice;
mod containers;
mod dataforge_cache;
mod definitions;
mod entity_loadout;
mod entries;
mod forge;
mod image_preview;
mod legal_notice;
mod model_animations;
mod models;
mod mtl_summary;
mod query;
mod read;
mod read_only;
mod read_text;
mod real_archive;
mod real_archive_model;
mod real_archive_query;
mod records;
mod reopen;
mod support;
mod tool_list;
