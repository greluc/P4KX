// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, BufWriter, Write};
use std::path::Path;

use anyhow::Result;
use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Armour, AssemblyOptions, BodyVariant, LoadoutResolver, PartOrigin,
};
use p4kx_extraction::{
    ConversionError, DataForgeEntry, GlbOptions, GlbSummary, GltfError, is_same_file,
    write_replacing,
};
use serde::Serialize;

use crate::armour_loadout::{Files, LoadoutJson, ReplacedJson, Selection};
use crate::cli_armour::ArmourGlbArgs;
use crate::entity_glb::{GlbOutputIsArchive, IssueJson, print_issues};
use crate::json;
use crate::query_table::one_line;

#[derive(Serialize)]
struct PartJson<'a> {
    name: &'a str,
    origin: &'static str,
    file: &'a str,
}

#[derive(Serialize)]
struct GlbJson<'a> {
    archive: String,
    body: &'static str,
    skeleton: &'a str,
    out: String,
    lod: u32,
    loadout: Option<&'a LoadoutJson<'a>>,
    pieces: usize,
    replaced: Vec<ReplacedJson<'a>>,
    parts: Vec<PartJson<'a>>,
    nodes: usize,
    bones: usize,
    meshes: usize,
    skinned_meshes: usize,
    primitives: usize,
    materials: usize,
    vertices: usize,
    triangles: usize,
    bytes: u64,
    skipped: usize,
    issues: Vec<IssueJson<'a>>,
}

/// Runs `p4kx armour glb`.
pub(crate) fn run(args: &ArmourGlbArgs, as_json: bool) -> Result<()> {
    if is_same_file(&args.out, &args.archive) {
        return Err(GlbOutputIsArchive(args.out.clone()).into());
    }
    let archive = crate::key::open(&args.archive)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = Files::open(ArchiveFiles::new(&archive), args.loadout.as_deref())?;
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let selection = Selection::gather(&mut resolver, &files, args)?;
    let variant = BodyVariant::from(args.body);
    let models = ArchiveModels::new(&archive);
    let options = AssemblyOptions::default().with_lod(args.lod);
    let armour = Armour::build(variant, &selection.pieces, &models, &options)?;
    let summary = write(&armour, args)?;
    if as_json {
        json::print(&GlbJson {
            archive: json::path(archive.path()),
            body: variant.code(),
            skeleton: armour.skeleton_file(),
            out: json::path(&args.out),
            lod: args.lod,
            loadout: selection.loadout.as_ref(),
            pieces: selection.pieces.len(),
            replaced: selection.replaced.iter().map(ReplacedJson::from).collect(),
            parts: armour
                .parts()
                .iter()
                .map(|part| PartJson {
                    name: part.name(),
                    origin: origin_code(part.origin()),
                    file: part.file(),
                })
                .collect(),
            nodes: summary.nodes,
            bones: summary.bones,
            meshes: summary.meshes,
            skinned_meshes: summary.skinned_meshes,
            primitives: summary.primitives,
            materials: summary.materials,
            vertices: summary.vertices,
            triangles: summary.triangles,
            bytes: summary.bytes,
            skipped: armour
                .issues()
                .iter()
                .filter(|issue| issue.kind().skips())
                .count(),
            issues: armour.issues().iter().map(IssueJson::from).collect(),
        })?;
        return Ok(());
    }
    match print_report(&armour, &summary, &selection, &args.out) {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(error.into()),
        _ => Ok(()),
    }
}

const fn origin_code(origin: PartOrigin) -> &'static str {
    match origin {
        PartOrigin::Body => "body",
        _ => "piece",
    }
}

fn write(armour: &Armour, args: &ArmourGlbArgs) -> Result<GlbSummary> {
    let mut options = GlbOptions::default().with_lod(args.lod);
    if args.no_vertex_colours {
        options = options.without_vertex_colours();
    }
    let out: &Path = &args.out;
    let name = armour.variant().code().to_owned();
    Ok(write_replacing(out, |writer| {
        armour
            .write_glb(&options, writer)
            .map_err(|error| match error {
                GltfError::Write(source) => ConversionError::Write {
                    path: out.to_path_buf(),
                    source,
                },
                source => ConversionError::Glb {
                    file: name.clone(),
                    source,
                },
            })
    })?)
}

fn print_report(
    armour: &Armour,
    summary: &GlbSummary,
    selection: &Selection<'_>,
    out: &Path,
) -> io::Result<()> {
    let mut stdout = BufWriter::new(io::stdout().lock());
    selection.print(&mut stdout)?;
    writeln!(
        stdout,
        "wrote {} on the {} body: {} pieces, {} parts ({} of the body), {} bones, {} nodes, {} meshes ({} skinned), {} primitives, {} materials, {} vertices, {} triangles, {} bytes",
        out.display(),
        armour.variant().code(),
        selection.pieces.len(),
        armour.parts().len(),
        armour.count(PartOrigin::Body),
        summary.bones,
        summary.nodes,
        summary.meshes,
        summary.skinned_meshes,
        summary.primitives,
        summary.materials,
        summary.vertices,
        summary.triangles,
        summary.bytes
    )?;
    for part in armour.parts() {
        writeln!(
            stdout,
            "part {}: {}",
            one_line(part.name()),
            one_line(part.file())
        )?;
    }
    if summary.lod_fallbacks > 0 {
        writeln!(
            stdout,
            "{} models lack LOD{} and are written at the nearest lower level",
            summary.lod_fallbacks,
            armour.lod()
        )?;
    }
    if summary.hidden_primitives > 0 {
        writeln!(
            stdout,
            "left out {} submeshes whose material draws nothing",
            summary.hidden_primitives
        )?;
    }
    print_issues(&mut stdout, armour.issues())?;
    stdout.flush()
}
