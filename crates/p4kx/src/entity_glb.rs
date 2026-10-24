// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, ForgePalettes, Issue, LoadoutChanges,
    LoadoutResolver,
};
use p4kx_extraction::{
    ConversionError, DataForgeEntry, GlbOptions, GlbSummary, GltfError, MaterialMode, is_same_file,
    write_replacing,
};
use serde::Serialize;
use thiserror::Error;

use crate::cli_entity::EntityGlbArgs;
use crate::cli_entity::EntityMaterialsArg;
use crate::json;
use crate::query_table::one_line;

/// The output of `p4kx entity glb` is the Archive it reads.
#[derive(Debug, Error)]
#[error("{} is the Archive the Entity is read from", .0.display())]
pub(crate) struct GlbOutputIsArchive(PathBuf);

#[derive(Serialize)]
struct GlbJson<'a> {
    archive: String,
    entity: &'a str,
    guid: String,
    out: String,
    lod: u32,
    parts: usize,
    modelled_parts: usize,
    palettes: usize,
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
    lod_fallbacks: usize,
    shared_meshes: usize,
    shared_materials: usize,
    shared_textures: usize,
    vertices: usize,
    triangles: usize,
    bytes: u64,
    skipped: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    excluded: Vec<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    replaced: Vec<ReplacedJson<'a>>,
    issues: Vec<IssueJson<'a>>,
}

#[derive(Serialize)]
struct FallbackJson<'a> {
    material: &'a str,
    role: &'static str,
    file: &'a str,
    reason: &'a str,
}

#[derive(Serialize)]
struct ReplacedJson<'a> {
    port: &'a str,
    entity: &'a str,
}

#[derive(Serialize)]
struct IssueJson<'a> {
    path: &'a str,
    name: &'a str,
    kind: &'static str,
    skipped: bool,
    reason: String,
}

impl<'a> From<&'a Issue> for IssueJson<'a> {
    fn from(issue: &'a Issue) -> Self {
        Self {
            path: issue.path(),
            name: issue.name(),
            kind: issue.kind().code(),
            skipped: issue.kind().skips(),
            reason: issue.kind().to_string(),
        }
    }
}

/// Runs `p4kx entity glb`.
#[expect(
    clippy::too_many_lines,
    reason = "the JSON report is one literal that names every field"
)]
pub(crate) fn run(args: &EntityGlbArgs, as_json: bool) -> Result<()> {
    if is_same_file(&args.out, &args.archive) {
        return Err(GlbOutputIsArchive(args.out.clone()).into());
    }
    let archive = crate::key::open(&args.archive)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge)
        .with_files(&files)
        .stopping_at_docked_vehicles();
    let changes = args.changes.changes()?;
    let record = resolver.entities().find(&args.entity)?;
    let tree = resolver.resolve_with(&record, &changes)?;
    let models = ArchiveModels::new(&archive);
    let options = AssemblyOptions::default().with_lod(args.lod);
    let palettes = ForgePalettes::new(&forge);
    let assembly = Assembly::build_with_palettes(&tree, &models, &palettes, &options)?;
    let summary = write(&assembly, &models, args)?;
    if as_json {
        json::print(&GlbJson {
            archive: json::path(archive.path()),
            entity: tree.class_name(),
            guid: tree.guid().to_string(),
            out: json::path(&args.out),
            lod: args.lod,
            parts: assembly.parts().len(),
            modelled_parts: assembly.modelled_parts(),
            palettes: painted(&assembly),
            nodes: summary.nodes,
            meshes: summary.meshes,
            primitives: summary.primitives,
            materials: summary.materials,
            textured_materials: summary.textured_materials,
            textures: summary.textures,
            texture_pixels: summary.texture_pixels,
            texture_fallbacks: fallbacks_json(&summary),
            default_material_ids: summary.default_material_ids,
            hidden_primitives: summary.hidden_primitives,
            lod_fallbacks: summary.lod_fallbacks,
            shared_meshes: summary.shared_meshes,
            shared_materials: summary.shared_materials,
            shared_textures: summary.shared_textures,
            vertices: summary.vertices,
            triangles: summary.triangles,
            bytes: summary.bytes,
            skipped: assembly
                .issues()
                .iter()
                .filter(|issue| issue.kind().skips())
                .count(),
            excluded: changes.exclusions().iter().map(String::as_str).collect(),
            replaced: changes
                .replacements()
                .iter()
                .map(|(port, entity)| ReplacedJson { port, entity })
                .collect(),
            issues: assembly.issues().iter().map(IssueJson::from).collect(),
        })?;
        return Ok(());
    }
    let printed = print_report(&assembly, &summary, &args.out, tree.class_name(), &changes);
    match printed {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(error.into()),
        _ => Ok(()),
    }
}

fn painted(assembly: &Assembly) -> usize {
    assembly
        .parts()
        .iter()
        .filter(|part| part.palette().is_some())
        .count()
}

fn fallbacks_json(summary: &GlbSummary) -> Vec<FallbackJson<'_>> {
    summary
        .texture_fallbacks
        .iter()
        .map(|fallback| FallbackJson {
            material: &fallback.material,
            role: fallback.role.name(),
            file: &fallback.file,
            reason: &fallback.reason,
        })
        .collect()
}

fn print_issues(out: &mut impl Write, assembly: &Assembly) -> io::Result<()> {
    for issue in assembly.issues() {
        let what = if issue.kind().skips() {
            "skipped"
        } else {
            "note"
        };
        let path = if issue.path().is_empty() {
            issue.name().to_owned()
        } else {
            format!("{} ({})", issue.path(), issue.name())
        };
        writeln!(
            out,
            "{what} {}: {}",
            one_line(&path),
            one_line(&issue.kind().to_string())
        )?;
    }
    Ok(())
}

fn write(
    assembly: &Assembly,
    models: &ArchiveModels<'_>,
    args: &EntityGlbArgs,
) -> Result<GlbSummary> {
    let mode = match args.materials {
        EntityMaterialsArg::Colours => MaterialMode::Colours,
        EntityMaterialsArg::Textures => MaterialMode::Textures,
    };
    let mut options = GlbOptions::default()
        .with_lod(args.lod)
        .with_materials(mode)
        .with_texture_mip(args.texture_mip);
    if args.no_vertex_colours {
        options = options.without_vertex_colours();
    }
    let name = assembly
        .root()
        .map_or_else(String::new, |root| root.class_name().to_owned());
    let out: &Path = &args.out;
    let textures = models.textures();
    Ok(write_replacing(out, |writer| {
        assembly
            .write_glb_with(&options, &textures, writer)
            .map_err(|error| match error {
                GltfError::Write(source) => ConversionError::Write {
                    path: out.to_path_buf(),
                    source,
                },
                source => ConversionError::Glb { file: name, source },
            })
    })?)
}

fn print_changes(out: &mut impl Write, changes: &LoadoutChanges) -> io::Result<()> {
    for port in changes.exclusions() {
        writeln!(out, "excluded {}", one_line(port))?;
    }
    for (port, entity) in changes.replacements() {
        writeln!(out, "replaced {} by {}", one_line(port), one_line(entity))?;
    }
    Ok(())
}

fn print_textures(out: &mut impl Write, summary: &GlbSummary) -> io::Result<()> {
    if summary.textures > 0 || !summary.texture_fallbacks.is_empty() {
        writeln!(
            out,
            "embedded {} images of {} pixels in {} of {} materials",
            summary.textures, summary.texture_pixels, summary.textured_materials, summary.materials
        )?;
    }
    for fallback in &summary.texture_fallbacks {
        writeln!(
            out,
            "no {} texture {} for material {}: {}; the plain colour is used",
            fallback.role,
            one_line(&fallback.file),
            one_line(&fallback.material),
            one_line(&fallback.reason)
        )?;
    }
    Ok(())
}

fn print_report(
    assembly: &Assembly,
    summary: &GlbSummary,
    out: &Path,
    class_name: &str,
    changes: &LoadoutChanges,
) -> io::Result<()> {
    let mut stdout = BufWriter::new(io::stdout().lock());
    writeln!(
        stdout,
        "wrote {} of {}: {} parts ({} with models), {} nodes, {} meshes, {} primitives, {} materials, {} vertices, {} triangles, {} bytes",
        out.display(),
        one_line(class_name),
        assembly.parts().len(),
        assembly.modelled_parts(),
        summary.nodes,
        summary.meshes,
        summary.primitives,
        summary.materials,
        summary.vertices,
        summary.triangles,
        summary.bytes
    )?;
    print_changes(&mut stdout, changes)?;
    let painted = painted(assembly);
    if painted > 0 {
        writeln!(stdout, "{painted} parts drawn with a tint palette")?;
    }
    print_textures(&mut stdout, summary)?;
    if summary.lod_fallbacks > 0 {
        writeln!(
            stdout,
            "{} models lack LOD{} and are written at the nearest lower level",
            summary.lod_fallbacks,
            assembly.lod()
        )?;
    }
    if summary.shared_meshes + summary.shared_materials + summary.shared_textures > 0 {
        writeln!(
            stdout,
            "shared {} meshes, {} materials and {} textures instead of writing them again",
            summary.shared_meshes, summary.shared_materials, summary.shared_textures
        )?;
    }
    if summary.hidden_primitives > 0 {
        writeln!(
            stdout,
            "left out {} submeshes whose material draws nothing",
            summary.hidden_primitives
        )?;
    }
    print_issues(&mut stdout, assembly)?;
    stdout.flush()
}
