// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::chunks::{Chunk, Container, KnownChunkType};
use p4kx_extraction::{
    AnimationChoice, AnimationSet, ChunkSource, ConversionError, MeshInfo, ModelInfo, ModelSource,
};
use serde::Serialize;

use crate::archives::Opened;
use crate::budget::{MAX_RESPONSE_LEN, fill, json_len};
use crate::error::ToolError;
use crate::model_params::Section;
use crate::model_shapes::{
    ChunkShape, ChunksResult, LodShape, ModelCounts, ModelItemsResult, ModelOverview,
};
use crate::page::Page;

/// The largest chunk file read for `list_chunks`, in bytes.
pub(crate) const MAX_CHUNK_FILE_LEN: u64 = 256 << 20;

/// Returns one page of the chunks of the chunk file Entry at `path`, those of the type `filter`
/// names if it is given.
pub(crate) fn chunks(
    opened: &Opened,
    path: &str,
    filter: Option<&str>,
    page: Page,
) -> Result<ChunksResult, ToolError> {
    let index = opened.index();
    let source = ChunkSource::from_index_limited(&index, path, MAX_CHUNK_FILE_LEN)
        .map_err(|error| converted(path, "a chunk file", error))?;
    let file = source.file();
    let container = file.container();
    let wanted = filter.map(|text| type_id(text, container)).transpose()?;
    let listed: Vec<(usize, &Chunk)> = file
        .chunks()
        .iter()
        .enumerate()
        .filter(|(_, chunk)| wanted.is_none_or(|id| chunk.chunk_type().id() == id))
        .collect();
    let items = page
        .slice(&listed)
        .iter()
        .map(|&(index, chunk)| Ok(chunk_shape(index, chunk)));
    let (chunks, next_cursor) = fill(page, listed.len(), items, |shape: &ChunkShape| {
        format!("chunk {}", shape.index)
    })?;
    Ok(ChunksResult {
        archive: opened.archive().path().display().to_string(),
        path: source.name().to_owned(),
        container: container.name(),
        container_version: container.version(),
        chunk_count: file.chunks().len(),
        unknown_count: file
            .chunks()
            .iter()
            .filter(|chunk| !chunk.chunk_type().is_known())
            .count(),
        total: listed.len(),
        chunks,
        next_cursor,
    })
}

fn chunk_shape(index: usize, chunk: &Chunk) -> ChunkShape {
    let chunk_type = chunk.chunk_type();
    ChunkShape {
        index,
        type_name: chunk_type.name(),
        type_id: chunk_type.id(),
        type_id_hex: chunk_type.hex_id(),
        known: chunk_type.is_known(),
        version: chunk.version(),
        id: chunk.id(),
        byte_order: chunk.byte_order().name(),
        offset: chunk.offset(),
        size: chunk.size(),
    }
}

fn type_id(text: &str, container: Container) -> Result<u32, ToolError> {
    let text = text.trim();
    let bad = || ToolError::BadChunkType(text.to_owned());
    if let Some(digits) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        return u32::from_str_radix(digits, 16).map_err(|_| bad());
    }
    KnownChunkType::table(container)
        .iter()
        .find(|known| known.name.eq_ignore_ascii_case(text))
        .map(|known| known.id)
        .ok_or_else(bad)
}

/// Returns the summary of the model Entry at `path`.
pub(crate) fn overview(opened: &Opened, path: &str) -> Result<ModelOverview, ToolError> {
    let (source, animations) = read(opened, path, true)?;
    let info = ModelInfo::of(&source, &animations);
    let library = serde_json::to_value(&info.library)?;
    let overview = ModelOverview {
        archive: opened.archive().path().display().to_string(),
        model: info.model.to_owned(),
        geometry: info.geometry.map(str::to_owned),
        material_library: info.material_library.map(str::to_owned),
        material_ids: info.material_ids,
        library,
        counts: ModelCounts {
            materials: info.materials.len(),
            nodes: info.nodes.len(),
            lods: info.lods.len(),
            meshes: info.lods.iter().map(|lod| lod.meshes.len()).sum(),
            bones: info.bones.len(),
            animations: info.animations.len(),
            unread_animations: info.unread_animations.len(),
            chunks: info.chunks.len(),
        },
        lods: info
            .lods
            .iter()
            .map(|lod| LodShape {
                level: lod.level,
                vertices: lod.vertices,
                triangles: lod.triangles,
                meshes: lod.meshes.len(),
            })
            .collect(),
    };
    if json_len(&overview) > MAX_RESPONSE_LEN {
        return Err(ToolError::TooLargeItem {
            what: format!("summary of the model {}", overview.model),
            limit: MAX_RESPONSE_LEN,
            hint: "its material IDs or levels of detail are too many for one response",
        });
    }
    Ok(overview)
}

/// Returns one page of the section `section` of the model Entry at `path`.
pub(crate) fn items(
    opened: &Opened,
    path: &str,
    section: Section,
    page: Page,
) -> Result<ModelItemsResult, ToolError> {
    let (source, animations) = read(opened, path, section.needs_animations())?;
    let info = ModelInfo::of(&source, &animations);
    let what = section.name();
    let (total, items, next_cursor) = match section {
        Section::Materials => paged(&info.materials, page, what)?,
        Section::Nodes => paged(&info.nodes, page, what)?,
        Section::Meshes => {
            let meshes: Vec<LodMesh<'_>> = info
                .lods
                .iter()
                .flat_map(|lod| {
                    lod.meshes.iter().map(|mesh| LodMesh {
                        lod: lod.level,
                        mesh,
                    })
                })
                .collect();
            paged(&meshes, page, what)?
        }
        Section::Bones => paged(&info.bones, page, what)?,
        Section::Animations => paged(&info.animations, page, what)?,
        Section::UnreadAnimations => paged(&info.unread_animations, page, what)?,
        Section::Chunks => paged(&info.chunks, page, what)?,
    };
    Ok(ModelItemsResult {
        archive: opened.archive().path().display().to_string(),
        model: info.model.to_owned(),
        section: what,
        total,
        items,
        next_cursor,
    })
}

#[derive(Serialize)]
struct LodMesh<'a> {
    lod: u32,
    #[serde(flatten)]
    mesh: &'a MeshInfo,
}

type Paged = (usize, Vec<serde_json::Value>, Option<String>);

fn paged<T: Serialize>(items: &[T], page: Page, what: &str) -> Result<Paged, ToolError> {
    let values = page
        .slice(items)
        .iter()
        .map(|item| serde_json::to_value(item).map_err(ToolError::from));
    let (values, next) = fill(page, items.len(), values, |_| format!("item of {what}"))?;
    Ok((items.len(), values, next))
}

fn read(
    opened: &Opened,
    path: &str,
    with_animations: bool,
) -> Result<(ModelSource, AnimationSet), ToolError> {
    let index = opened.index();
    let source =
        ModelSource::from_index(&index, path).map_err(|error| converted(path, "a model", error))?;
    let animations = if with_animations {
        AnimationSet::from_archive(&index, source.name(), &AnimationChoice::All, &[])
            .map_err(|error| converted(path, "a model with its animations", error))?
    } else {
        AnimationSet::default()
    };
    Ok((source, animations))
}

fn converted(path: &str, what: &'static str, error: ConversionError) -> ToolError {
    if matches!(error, ConversionError::UnknownEntry(_)) {
        ToolError::UnknownEntry(path.to_owned())
    } else {
        ToolError::Convert {
            path: path.to_owned(),
            what,
            source: Box::new(error),
        }
    }
}
