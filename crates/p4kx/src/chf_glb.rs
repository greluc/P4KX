// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, BufWriter, Write};
use std::path::Path;

use anyhow::Result;
use p4kx_assembly::character::{CHF_SIZE, Chf};
use p4kx_assembly::{ArchiveModels, Armour, AssemblyOptions, Head, PartOrigin};
use p4kx_extraction::{
    ConversionError, GlbOptions, GlbSummary, GltfError, is_same_file, write_replacing,
};
use serde::Serialize;

use crate::chf::{InvalidChf, read_capped};
use crate::cli::ChfGlbArgs;
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
    chf: String,
    body: &'static str,
    rig: &'static str,
    skeleton: &'a str,
    out: String,
    lod: u32,
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

const fn origin_code(origin: PartOrigin) -> &'static str {
    match origin {
        PartOrigin::Body => "body",
        _ => "head",
    }
}

/// Runs `p4kx chf glb`.
pub(crate) fn run(args: &ChfGlbArgs, as_json: bool) -> Result<()> {
    if is_same_file(&args.out, &args.archive) {
        return Err(GlbOutputIsArchive(args.out.clone()).into());
    }
    let bytes = read_capped(&args.file, CHF_SIZE as u64)?;
    let chf = Chf::read(&bytes).map_err(|source| InvalidChf {
        path: args.file.clone(),
        source,
    })?;
    let head = Head::from_character(&chf.character)?;
    let archive = crate::key::open(&args.archive)?;
    let models = ArchiveModels::new(&archive);
    let options = AssemblyOptions::default().with_lod(args.lod);
    let armour = Armour::build(head.variant(), head.pieces(), &models, &options)?;

    let mut glb = GlbOptions::default().with_lod(args.lod);
    if args.no_vertex_colours {
        glb = glb.without_vertex_colours();
    }
    let name = head.variant().code().to_owned();
    let summary = write_replacing(&args.out, |writer| {
        armour.write_glb(&glb, writer).map_err(|error| match error {
            GltfError::Write(source) => ConversionError::Write {
                path: args.out.clone(),
                source,
            },
            source => ConversionError::Glb {
                file: name.clone(),
                source,
            },
        })
    })?;

    if as_json {
        json::print(&report(archive.path(), args, &head, &armour, &summary))?;
        return Ok(());
    }
    match print_report(&armour, &head, &summary, args) {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(error.into()),
        _ => Ok(()),
    }
}

fn report<'a>(
    archive: &Path,
    args: &ChfGlbArgs,
    head: &Head,
    armour: &'a Armour,
    summary: &GlbSummary,
) -> GlbJson<'a> {
    GlbJson {
        archive: json::path(archive),
        chf: json::path(&args.file),
        body: head.variant().code(),
        rig: head.rig(),
        skeleton: armour.skeleton_file(),
        out: json::path(&args.out),
        lod: args.lod,
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
    }
}

fn print_report(
    armour: &Armour,
    head: &Head,
    summary: &GlbSummary,
    args: &ChfGlbArgs,
) -> io::Result<()> {
    let mut stdout = BufWriter::new(io::stdout().lock());
    writeln!(
        stdout,
        "wrote {} with the head of {} on the {} body: {} parts, {} bones, {} nodes, {} meshes ({} skinned), {} primitives, {} materials, {} vertices, {} triangles, {} bytes",
        args.out.display(),
        head.rig(),
        head.variant().code(),
        armour.parts().len(),
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
    print_issues(&mut stdout, armour.issues())?;
    stdout.flush()
}
