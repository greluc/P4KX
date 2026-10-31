// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::chunks::{Chunk, Container, KnownChunkType};
use p4kx_extraction::model::LodSelection;
use p4kx_extraction::{
    AnimationChoice, AnimationInfo, AnimationSet, BoneInfo, ChunkInfo, ChunkJson, ChunkSource,
    ConversionError, LibraryInfo, LodInfo, MaterialInfo, MeshInfo, ModelSource, NodeInfo,
    UnreadInfo,
};
use serde::Serialize;

use crate::archives::Opened;
use crate::budget::{MAX_RESPONSE_LEN, fill_within, json_len};
use crate::error::ToolError;
use crate::model_params::Section;
use crate::model_shapes::{
    ChunkShape, ChunksResult, LodShape, ModelCounts, ModelItemsResult, ModelOverview,
};
use crate::page::Page;

/// Returns one page of the chunks of the chunk file Entry at `path`, those of the type `filter`
/// names if it is given.
pub(crate) fn chunks(
    opened: &Opened,
    path: &str,
    filter: Option<&str>,
    page: Page,
) -> Result<ChunksResult, ToolError> {
    let index = opened.index();
    let source = ChunkSource::from_index(&index, path)
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
    let mut result = ChunksResult {
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
        chunks: Vec::new(),
        next_cursor: None,
    };
    let (chunks, next_cursor) = fill_within(
        page,
        listed.len(),
        json_len(&result),
        items,
        |shape: &ChunkShape| format!("chunk {}", shape.chunk.index),
    )?;
    result.chunks = chunks;
    result.next_cursor = next_cursor;
    Ok(result)
}

fn chunk_shape(index: usize, chunk: &Chunk) -> ChunkShape {
    ChunkShape {
        chunk: ChunkJson::of(index, chunk),
        type_id_hex: chunk.chunk_type().hex_id(),
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
    let model = source.model();
    let library = serde_json::to_value(LibraryInfo::of(model))?;
    let overview = ModelOverview {
        archive: opened.archive().path().display().to_string(),
        model: source.name().to_owned(),
        geometry: model.geometry.clone(),
        material_library: model.material_library.clone(),
        material_ids: model.material_ids(),
        library,
        counts: ModelCounts {
            materials: model
                .materials
                .library()
                .map_or(0, |library| library.materials.len()),
            nodes: model.nodes.len(),
            lods: model.lods.len(),
            meshes: model.lods.iter().map(|lod| lod.meshes.len()).sum(),
            bones: model.bones.len(),
            animations: animations.animations.len(),
            unread_animations: animations.unread.len(),
            chunks: model.chunks.len(),
        },
        lods: model
            .lods
            .iter()
            .map(|lod| LodShape {
                level: lod.level,
                vertices: lod.vertex_count(),
                triangles: lod.triangle_count(),
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

/// Returns one page of the section `section` of the model Entry at `path`; only that section is
/// built, and the animations are read only for the two sections that list them.
pub(crate) fn items(
    opened: &Opened,
    path: &str,
    section: Section,
    page: Page,
) -> Result<ModelItemsResult, ToolError> {
    let (source, animations) = read(opened, path, section.needs_animations())?;
    let model = source.model();
    let what = section.name();
    let reserved = envelope_len(opened, source.name(), what);
    let (total, items, next_cursor) = match section {
        Section::Materials => paged(
            &MaterialInfo::of_model(model),
            page,
            what,
            reserved,
            |item| item,
        )?,
        Section::Nodes => paged(&NodeInfo::of_model(model), page, what, reserved, |item| {
            item
        })?,
        Section::Meshes => {
            let lods = LodInfo::of_model(model);
            let meshes: Vec<LodMesh<'_>> = lods
                .iter()
                .flat_map(|lod| {
                    lod.meshes.iter().map(|mesh| LodMesh {
                        lod: lod.level,
                        mesh,
                    })
                })
                .collect();
            paged(&meshes, page, what, reserved, |item| item)?
        }
        Section::Bones => paged(&BoneInfo::of_model(model), page, what, reserved, |item| {
            item
        })?,
        Section::Animations => {
            let read: Vec<_> = animations
                .animations
                .iter()
                .zip(&animations.origins)
                .collect();
            paged(&read, page, what, reserved, |&(animation, origin)| {
                AnimationInfo::of(model, animation, origin)
            })?
        }
        Section::UnreadAnimations => {
            paged(&animations.unread, page, what, reserved, UnreadInfo::of)?
        }
        Section::Chunks => paged(&ChunkInfo::of_model(model), page, what, reserved, |item| {
            item
        })?,
    };
    Ok(ModelItemsResult {
        archive: opened.archive().path().display().to_string(),
        model: source.name().to_owned(),
        section: what,
        total,
        items,
        next_cursor,
    })
}

fn envelope_len(opened: &Opened, model: &str, section: &str) -> usize {
    json_len(&(
        opened.archive().path().display().to_string(),
        model,
        section,
    ))
    .saturating_add(128)
}

#[derive(Serialize)]
struct LodMesh<'a> {
    lod: u32,
    #[serde(flatten)]
    mesh: &'a MeshInfo,
}

type Paged = (usize, Vec<serde_json::Value>, Option<String>);

fn paged<'a, T, U: Serialize>(
    items: &'a [T],
    page: Page,
    what: &str,
    reserved: usize,
    convert: impl Fn(&'a T) -> U,
) -> Result<Paged, ToolError> {
    let values = page
        .slice(items)
        .iter()
        .map(|item| serde_json::to_value(convert(item)).map_err(ToolError::from));
    let (values, next) = fill_within(page, items.len(), reserved, values, |_| {
        format!("item of {what}")
    })?;
    Ok((items.len(), values, next))
}

fn read(
    opened: &Opened,
    path: &str,
    with_animations: bool,
) -> Result<(ModelSource, AnimationSet), ToolError> {
    let index = opened.index();
    let source = ModelSource::from_index_plain_materials(&index, path, LodSelection::All)
        .map_err(|error| converted(path, "a model", error))?;
    let animations = if with_animations {
        AnimationSet::from_archive(&index, source.name(), &AnimationChoice::All, &[])
            .map_err(|error| converted(path, "a model with its animations", error))?
    } else {
        AnimationSet::default()
    };
    Ok((source, animations))
}

fn converted(path: &str, what: &'static str, error: ConversionError) -> ToolError {
    match error {
        ConversionError::UnknownEntry(unknown) => ToolError::UnknownEntry(
            unknown
                .0
                .first()
                .map_or_else(|| path.to_owned(), Clone::clone),
        ),
        error => ToolError::Convert {
            path: path.to_owned(),
            what,
            source: Box::new(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use p4kx_extraction::{ConversionError, UnknownEntries};

    use super::converted;
    use crate::error::ToolError;

    #[test]
    fn an_unknown_entry_error_names_the_path_it_carries() {
        let error = ConversionError::from(UnknownEntries(vec!["Data/other.cgfm".to_owned()]));
        assert!(
            matches!(
                converted("Data/model.cgf", "a model", error),
                ToolError::UnknownEntry(path) if path == "Data/other.cgfm"
            ),
            "the missing Entry, not the model"
        );
    }
}
