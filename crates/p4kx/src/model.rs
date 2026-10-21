use std::borrow::Cow;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use p4kx_extraction::model::{LodSelection, Model, ModelError};
use p4kx_extraction::{
    AnimationChoice, AnimationSet, ConversionError, GlbOptions, GlbSummary, GltfError,
    MaterialMode, ModelSource, PathIndex,
};
use serde::Serialize;

use crate::cli_model::{MaterialsArg, ModelAction, ModelArgs, ModelFileSource};
use crate::json;
use crate::model_animations::{UnreadJson, WrittenJson, write_notes as write_animation_notes};
use crate::model_info::info;
use crate::model_json::{ChunkJson, file_name};
use crate::model_materials_json::LibraryJson;

/// Returns `text` with every control character replaced by U+FFFD, so that a name read from a
/// model cannot drive the terminal.
pub(crate) fn clean(text: &str) -> Cow<'_, str> {
    if text.chars().any(char::is_control) {
        Cow::Owned(
            text.chars()
                .map(|c| if c.is_control() { '\u{fffd}' } else { c })
                .collect(),
        )
    } else {
        Cow::Borrowed(text)
    }
}

#[derive(Serialize)]
struct GlbJson<'a> {
    model: &'a str,
    out: String,
    nodes: usize,
    meshes: usize,
    primitives: usize,
    materials: usize,
    default_material_ids: usize,
    hidden_primitives: usize,
    bones: usize,
    skinned_meshes: usize,
    vertices: usize,
    triangles: usize,
    bytes: u64,
    library: LibraryJson<'a>,
    animations: Vec<WrittenJson>,
    unread_animations: Vec<UnreadJson>,
    unknown_chunks: Vec<ChunkJson>,
}

pub(crate) type Opened = (ModelSource, AnimationSet);

/// Runs `p4kx model info` or `p4kx model glb`.
pub(crate) fn run(args: &ModelArgs, as_json: bool) -> Result<()> {
    match &args.action {
        ModelAction::Info(source) => info(
            &open(source, LodSelection::All, Some(&AnimationChoice::All))?,
            as_json,
        ),
        ModelAction::Glb {
            source,
            out,
            lod,
            materials,
            no_vertex_colours,
            animations,
        } => {
            let choice = choice_of(animations);
            let opened = open(source, LodSelection::Only(*lod), choice.as_ref())?;
            let mode = match materials {
                MaterialsArg::Colours => MaterialMode::Colours,
            };
            let options = GlbOptions::default().with_lod(*lod).with_materials(mode);
            let options = if *no_vertex_colours {
                options.without_vertex_colours()
            } else {
                options
            };
            glb(&opened, options, out, as_json)
        }
    }
}

fn choice_of(animations: &[String]) -> Option<AnimationChoice> {
    if animations.is_empty() {
        None
    } else if animations
        .iter()
        .any(|name| name.eq_ignore_ascii_case("all"))
    {
        Some(AnimationChoice::All)
    } else {
        Some(AnimationChoice::Named(animations.to_vec()))
    }
}

fn open(
    source: &ModelFileSource,
    lods: LodSelection,
    choice: Option<&AnimationChoice>,
) -> Result<Opened> {
    if let Some(entry) = &source.entry {
        let archive = crate::key::open(&source.path)?;
        let index = PathIndex::new(&archive);
        let model = ModelSource::from_index_with(&index, entry, lods)?;
        let animations = choice
            .map(|choice| {
                AnimationSet::from_archive(&index, entry, choice, &source.animation_sources)
            })
            .transpose()?
            .unwrap_or_default();
        return Ok((model, animations));
    }
    let model = ModelSource::from_file_with(&source.path, lods)?;
    let extra: Vec<PathBuf> = source.animation_sources.iter().map(PathBuf::from).collect();
    let animations = choice
        .map(|choice| AnimationSet::from_disk(&source.path, choice, &extra))
        .transpose()?
        .unwrap_or_default();
    Ok((model, animations))
}
fn glb(
    (source, animations): &Opened,
    options: GlbOptions,
    out: &Path,
    as_json: bool,
) -> Result<()> {
    let summary = source.write_glb_animated(animations, &options, out)?;
    let model = source.model();
    if as_json {
        json::print(&glb_json(source, animations, out, &summary))?;
        return Ok(());
    }
    let mut stdout = BufWriter::new(io::stdout().lock());
    writeln!(
        stdout,
        "wrote {} of {}: {} nodes, {} meshes, {} primitives, {} materials, {} vertices, {} triangles, {} bytes",
        out.display(),
        clean(source.name()),
        summary.nodes,
        summary.meshes,
        summary.primitives,
        summary.materials,
        summary.vertices,
        summary.triangles,
        summary.bytes
    )?;
    write_notes(&mut stdout, model, &summary)?;
    write_animation_notes(&mut stdout, &summary.animations, animations)?;
    stdout.flush()?;
    Ok(())
}

fn write_notes(stdout: &mut impl Write, model: &Model, summary: &GlbSummary) -> io::Result<()> {
    if summary.bones > 0 {
        writeln!(
            stdout,
            "{} bones, {} skinned meshes",
            summary.bones, summary.skinned_meshes
        )?;
    }
    if let Some(problem) = model.materials.problem() {
        writeln!(stdout, "{}; the default material is used", clean(&problem))?;
    }
    if summary.default_material_ids > 0 && model.materials.problem().is_none() {
        writeln!(
            stdout,
            "{} material IDs have no sub-material in the library; the default material is used",
            summary.default_material_ids
        )?;
    }
    if let Some(library) = model.materials.library()
        && library.unknown_entries > 0
    {
        writeln!(
            stdout,
            "{} entries of the material library are no material; the default material is used for them",
            library.unknown_entries
        )?;
    }
    if summary.hidden_primitives > 0 {
        writeln!(
            stdout,
            "left out {} submeshes whose material draws nothing",
            summary.hidden_primitives
        )?;
    }
    for chunk in model.unknown_chunks() {
        writeln!(
            stdout,
            "skipped unknown chunk {} {} of the {} file",
            chunk.index,
            chunk.chunk_type.hex_id(),
            file_name(chunk.file)
        )?;
    }
    Ok(())
}

fn glb_json<'a>(
    source: &'a ModelSource,
    animations: &AnimationSet,
    out: &Path,
    summary: &GlbSummary,
) -> GlbJson<'a> {
    GlbJson {
        model: source.name(),
        out: json::path(out),
        nodes: summary.nodes,
        meshes: summary.meshes,
        primitives: summary.primitives,
        materials: summary.materials,
        default_material_ids: summary.default_material_ids,
        hidden_primitives: summary.hidden_primitives,
        bones: summary.bones,
        skinned_meshes: summary.skinned_meshes,
        vertices: summary.vertices,
        triangles: summary.triangles,
        bytes: summary.bytes,
        library: LibraryJson::of(source.model()),
        animations: summary.animations.iter().map(WrittenJson::of).collect(),
        unread_animations: animations.unread.iter().map(UnreadJson::of).collect(),
        unknown_chunks: source.model().unknown_chunks().map(ChunkJson::of).collect(),
    }
}

/// Returns whether `error` is a model failure caused by the command line: a geometry companion
/// that does not exist or a LOD the model does not have.
pub(crate) fn is_invalid_input(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<ConversionError>(),
        Some(
            ConversionError::Model {
                source: ModelError::MissingGeometry { .. } | ModelError::NoSuchLod { .. },
                ..
            } | ConversionError::UnknownAnimations { .. }
                | ConversionError::NotAnimationFile { .. }
                | ConversionError::Glb {
                    source: GltfError::NoSuchLod { .. },
                    ..
                }
        )
    )
}

/// Returns whether `error` is a model failure caused by an unreadable or corrupt model or
/// companion.
pub(crate) fn is_archive_error(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<ConversionError>(),
        Some(
            ConversionError::Model {
                source: ModelError::Companion { .. }
                    | ModelError::CompanionChunks { .. }
                    | ModelError::NoMesh { .. }
                    | ModelError::Corrupt { .. },
                ..
            } | ConversionError::Animation { .. }
        )
    )
}
