use std::io::{self, BufWriter, Write};
use std::path::Path;

use anyhow::Result;
use p4kx_extraction::model::{ChunkUsage, Model, ModelError};
use p4kx_extraction::{Archive, ConversionError, GlbOptions, GlbSummary, GltfError, ModelSource};
use serde::Serialize;

use crate::cli_model::{ModelAction, ModelArgs, ModelFileSource};
use crate::json;
use crate::model_json::{ChunkJson, InfoJson, file_name, node_name, usage_name};

const NONE: &str = "-";

#[derive(Serialize)]
struct GlbJson<'a> {
    model: &'a str,
    out: String,
    nodes: usize,
    meshes: usize,
    primitives: usize,
    materials: usize,
    vertices: usize,
    triangles: usize,
    bytes: u64,
    unknown_chunks: Vec<ChunkJson>,
}

/// Runs `p4kx model info` or `p4kx model glb`.
pub(crate) fn run(args: &ModelArgs, as_json: bool) -> Result<()> {
    match &args.action {
        ModelAction::Info(source) => info(&open(source)?, as_json),
        ModelAction::Glb { source, out } => glb(&open(source)?, out, as_json),
    }
}

fn open(source: &ModelFileSource) -> Result<ModelSource> {
    Ok(match &source.entry {
        Some(entry) => ModelSource::from_archive(&Archive::open(&source.path)?, entry)?,
        None => ModelSource::from_file(&source.path)?,
    })
}

fn info(source: &ModelSource, as_json: bool) -> Result<()> {
    if as_json {
        json::print(&InfoJson::of(source))?;
        return Ok(());
    }
    let model = source.model();
    let mut out = BufWriter::new(io::stdout().lock());
    writeln!(out, "Model:       {}", source.name())?;
    writeln!(
        out,
        "Geometry:    {}",
        model.geometry.as_deref().unwrap_or(NONE)
    )?;
    writeln!(out, "Nodes:       {}", model.nodes.len())?;
    writeln!(out, "LODs:        {}", model.lods.len())?;
    writeln!(out, "Materials:   {}", materials(model))?;
    writeln!(out, "Bones:       {}", model.bones.len())?;
    writeln!(out, "Chunks:      {}", chunk_counts(model))?;
    write_lods(&mut out, model)?;
    write_nodes(&mut out, model)?;
    write_chunks(&mut out, model)?;
    out.flush()?;
    Ok(())
}

fn write_lods(out: &mut impl Write, model: &Model) -> io::Result<()> {
    writeln!(out)?;
    writeln!(
        out,
        "{:>4}  {:>10}  {:>10}  {:>6}",
        "LOD", "Vertices", "Triangles", "Meshes"
    )?;
    for lod in &model.lods {
        writeln!(
            out,
            "{:>4}  {:>10}  {:>10}  {:>6}",
            lod.level,
            lod.vertex_count(),
            lod.triangle_count(),
            lod.meshes.len()
        )?;
    }
    writeln!(out)?;
    writeln!(
        out,
        "{:>4}  {:<24}  {:>10}  {:>10}  {:<7}  Material IDs",
        "LOD", "Node", "Vertices", "Triangles", "Normals"
    )?;
    for lod in &model.lods {
        for mesh in &lod.meshes {
            let ids: Vec<String> = mesh
                .submeshes
                .iter()
                .map(|submesh| submesh.material_id.to_string())
                .collect();
            writeln!(
                out,
                "{:>4}  {:<24}  {:>10}  {:>10}  {:<7}  {}",
                lod.level,
                node_name(model, mesh.node),
                mesh.positions.len(),
                mesh.indices.len() / 3,
                if mesh.normals.is_some() { "yes" } else { "no" },
                ids.join(", ")
            )?;
        }
    }
    Ok(())
}

fn write_nodes(out: &mut impl Write, model: &Model) -> io::Result<()> {
    writeln!(out)?;
    writeln!(out, "{:>5}  {:<6}  Name", "Node", "Parent")?;
    for (index, node) in model.nodes.iter().enumerate() {
        let parent = node
            .parent
            .map_or_else(|| NONE.to_owned(), |parent| parent.to_string());
        writeln!(out, "{index:>5}  {parent:<6}  {}", node_name(model, index))?;
    }
    Ok(())
}

fn write_chunks(out: &mut impl Write, model: &Model) -> io::Result<()> {
    writeln!(out)?;
    writeln!(
        out,
        "{:<8}  {:>5}  {:<24}  {:>10}  Usage",
        "File", "Index", "Type", "Type ID"
    )?;
    for chunk in &model.chunks {
        writeln!(
            out,
            "{:<8}  {:>5}  {:<24}  {:>10}  {}",
            file_name(chunk.file),
            chunk.index,
            chunk.chunk_type.name().unwrap_or("unknown"),
            chunk.chunk_type.hex_id(),
            usage_name(chunk.usage)
        )?;
    }
    Ok(())
}
fn materials(model: &Model) -> String {
    let ids: Vec<String> = model.material_ids().iter().map(u16::to_string).collect();
    let ids = if ids.is_empty() {
        "no material IDs".to_owned()
    } else {
        format!("IDs {}", ids.join(", "))
    };
    model.material_library.as_ref().map_or_else(
        || format!("no material library, {ids}"),
        |library| format!("{library}, {ids}"),
    )
}

fn chunk_counts(model: &Model) -> String {
    let count = |usage| {
        model
            .chunks
            .iter()
            .filter(|chunk| chunk.usage == usage)
            .count()
    };
    format!(
        "{} ({} read, {} skipped, {} unknown)",
        model.chunks.len(),
        count(ChunkUsage::Read),
        count(ChunkUsage::Skipped),
        count(ChunkUsage::Unknown)
    )
}

fn glb(source: &ModelSource, out: &Path, as_json: bool) -> Result<()> {
    let summary = source.write_glb(&GlbOptions::default(), out)?;
    let model = source.model();
    if as_json {
        json::print(&glb_json(source, out, &summary))?;
        return Ok(());
    }
    let mut stdout = BufWriter::new(io::stdout().lock());
    writeln!(
        stdout,
        "wrote {} of {}: {} nodes, {} meshes, {} primitives, {} materials, {} vertices, {} triangles, {} bytes",
        out.display(),
        source.name(),
        summary.nodes,
        summary.meshes,
        summary.primitives,
        summary.materials,
        summary.vertices,
        summary.triangles,
        summary.bytes
    )?;
    for chunk in model.unknown_chunks() {
        writeln!(
            stdout,
            "skipped unknown chunk {} {} of the {} file",
            chunk.index,
            chunk.chunk_type.hex_id(),
            file_name(chunk.file)
        )?;
    }
    stdout.flush()?;
    Ok(())
}

fn glb_json<'a>(source: &'a ModelSource, out: &Path, summary: &GlbSummary) -> GlbJson<'a> {
    GlbJson {
        model: source.name(),
        out: json::path(out),
        nodes: summary.nodes,
        meshes: summary.meshes,
        primitives: summary.primitives,
        materials: summary.materials,
        vertices: summary.vertices,
        triangles: summary.triangles,
        bytes: summary.bytes,
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
                source: ModelError::MissingGeometry { .. },
                ..
            } | ConversionError::Glb {
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
        Some(ConversionError::Model {
            source: ModelError::Companion { .. }
                | ModelError::CompanionChunks { .. }
                | ModelError::NoMesh { .. }
                | ModelError::Corrupt { .. },
            ..
        })
    )
}
