// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, Issue, LoadoutChanges, LoadoutResolver,
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
    nodes: usize,
    meshes: usize,
    primitives: usize,
    materials: usize,
    default_material_ids: usize,
    hidden_primitives: usize,
    lod_fallbacks: usize,
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
    let assembly = Assembly::build(&tree, &models, &options)?;
    let summary = write(&assembly, args)?;
    if as_json {
        json::print(&GlbJson {
            archive: json::path(archive.path()),
            entity: tree.class_name(),
            guid: tree.guid().to_string(),
            out: json::path(&args.out),
            lod: args.lod,
            parts: assembly.parts().len(),
            modelled_parts: assembly.modelled_parts(),
            nodes: summary.nodes,
            meshes: summary.meshes,
            primitives: summary.primitives,
            materials: summary.materials,
            default_material_ids: summary.default_material_ids,
            hidden_primitives: summary.hidden_primitives,
            lod_fallbacks: summary.lod_fallbacks,
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

fn write(assembly: &Assembly, args: &EntityGlbArgs) -> Result<GlbSummary> {
    let mode = match args.materials {
        EntityMaterialsArg::Colours => MaterialMode::Colours,
    };
    let mut options = GlbOptions::default()
        .with_lod(args.lod)
        .with_materials(mode);
    if args.no_vertex_colours {
        options = options.without_vertex_colours();
    }
    let name = assembly
        .root()
        .map_or_else(String::new, |root| root.class_name().to_owned());
    let out: &Path = &args.out;
    Ok(write_replacing(out, |writer| {
        assembly
            .write_glb(&options, writer)
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
    if summary.lod_fallbacks > 0 {
        writeln!(
            stdout,
            "{} models lack LOD{} and are written at the nearest lower level",
            summary.lod_fallbacks,
            assembly.lod()
        )?;
    }
    if summary.hidden_primitives > 0 {
        writeln!(
            stdout,
            "left out {} submeshes whose material draws nothing",
            summary.hidden_primitives
        )?;
    }
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
            stdout,
            "{what} {}: {}",
            one_line(&path),
            one_line(&issue.kind().to_string())
        )?;
    }
    stdout.flush()
}
