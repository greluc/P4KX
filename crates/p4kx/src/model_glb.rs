use std::io::{self, BufWriter, Write};
use std::path::Path;

use anyhow::Result;
use p4kx_extraction::model::{LodSelection, Model};
use p4kx_extraction::{GlbOptions, GlbSummary, MaterialMode, ModelSource, PathIndex};
use serde::Serialize;

use crate::cli_model::{MaterialsArg, ModelAction};
use crate::json;
use crate::model_json::{ChunkJson, file_name};
use crate::model_materials_json::LibraryJson;
use crate::model_text::printable;

#[derive(Serialize)]
struct GlbJson<'a> {
    model: &'a str,
    out: String,
    nodes: usize,
    meshes: usize,
    primitives: usize,
    materials: usize,
    textured_materials: usize,
    textures: usize,
    texture_pixels: u64,
    texture_fallbacks: Vec<FallbackJson<'a>>,
    default_material_ids: usize,
    hidden_primitives: usize,
    vertices: usize,
    triangles: usize,
    bytes: u64,
    library: LibraryJson<'a>,
    unknown_chunks: Vec<ChunkJson>,
}

#[derive(Serialize)]
struct FallbackJson<'a> {
    material: &'a str,
    role: &'static str,
    file: &'a str,
    reason: &'a str,
}

/// Runs `p4kx model glb`.
pub(crate) fn run(action: &ModelAction, as_json: bool) -> Result<()> {
    let ModelAction::Glb {
        source,
        out,
        lod,
        materials,
        texture_mip,
        no_vertex_colours,
    } = action
    else {
        return Ok(());
    };
    let mode = match materials {
        MaterialsArg::Colours => MaterialMode::Colours,
        MaterialsArg::Textures => MaterialMode::Textures,
    };
    let mut options = GlbOptions::default()
        .with_lod(*lod)
        .with_materials(mode)
        .with_texture_mip(*texture_mip);
    if *no_vertex_colours {
        options = options.without_vertex_colours();
    }
    let lods = LodSelection::Only(*lod);
    if let Some(entry) = &source.entry {
        let archive = crate::key::open(&source.path)?;
        let index = PathIndex::new(&archive);
        let model = ModelSource::from_index_with(&index, entry, lods)?;
        let summary = model.write_glb_textured(&index, &options, out)?;
        report(&model, &options, &summary, out, true, as_json)
    } else {
        let model = ModelSource::from_file_with(&source.path, lods)?;
        let summary = model.write_glb(&options, out)?;
        report(&model, &options, &summary, out, false, as_json)
    }
}

fn report(
    source: &ModelSource,
    options: &GlbOptions,
    summary: &GlbSummary,
    out: &Path,
    from_archive: bool,
    as_json: bool,
) -> Result<()> {
    let model = source.model();
    if as_json {
        json::print(&glb_json(source, out, summary))?;
        return Ok(());
    }
    let mut stdout = BufWriter::new(io::stdout().lock());
    writeln!(
        stdout,
        "wrote {} of {}: {} nodes, {} meshes, {} primitives, {} materials, {} vertices, {} triangles, {} bytes",
        printable(&out.display().to_string()),
        printable(source.name()),
        summary.nodes,
        summary.meshes,
        summary.primitives,
        summary.materials,
        summary.vertices,
        summary.triangles,
        summary.bytes
    )?;
    if options.materials == MaterialMode::Textures {
        write_textures(&mut stdout, summary, from_archive)?;
    }
    write_notes(&mut stdout, model, summary)?;
    stdout.flush()?;
    Ok(())
}

fn write_notes(out: &mut impl Write, model: &Model, summary: &GlbSummary) -> io::Result<()> {
    let stdout = out;
    if let Some(problem) = model.materials.problem() {
        writeln!(
            stdout,
            "{}; the default material is used",
            printable(&problem)
        )?;
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

fn write_textures(
    out: &mut impl Write,
    summary: &GlbSummary,
    from_archive: bool,
) -> io::Result<()> {
    if !from_archive {
        return writeln!(
            out,
            "textures are read from an Archive; give the model as ARCHIVE ENTRY. The materials keep their plain colours"
        );
    }
    writeln!(
        out,
        "embedded {} images of {} pixels in {} of {} materials",
        summary.textures, summary.texture_pixels, summary.textured_materials, summary.materials
    )?;
    for fallback in &summary.texture_fallbacks {
        writeln!(
            out,
            "no {} texture {} for material {}: {}; the plain colour is used",
            fallback.role,
            printable(&fallback.file),
            printable(&fallback.material),
            printable(&fallback.reason)
        )?;
    }
    Ok(())
}

fn glb_json<'a>(source: &'a ModelSource, out: &Path, summary: &'a GlbSummary) -> GlbJson<'a> {
    GlbJson {
        model: source.name(),
        out: json::path(out),
        nodes: summary.nodes,
        meshes: summary.meshes,
        primitives: summary.primitives,
        materials: summary.materials,
        textured_materials: summary.textured_materials,
        textures: summary.textures,
        texture_pixels: summary.texture_pixels,
        texture_fallbacks: summary
            .texture_fallbacks
            .iter()
            .map(|fallback| FallbackJson {
                material: &fallback.material,
                role: fallback.role.name(),
                file: &fallback.file,
                reason: &fallback.reason,
            })
            .collect(),
        default_material_ids: summary.default_material_ids,
        hidden_primitives: summary.hidden_primitives,
        vertices: summary.vertices,
        triangles: summary.triangles,
        bytes: summary.bytes,
        library: LibraryJson::of(source.model()),
        unknown_chunks: source.model().unknown_chunks().map(ChunkJson::of).collect(),
    }
}
