// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use p4kx_assembly::character::{CHF_SIZE, Chf};
use p4kx_assembly::{
    ArchiveModels, Armour, AssemblyOptions, Expressions, Head, IssueKind, PartOrigin,
};
use p4kx_extraction::{
    ConversionError, GlbOptions, GlbSummary, GltfError, is_same_file, write_replacing,
};
use serde::Serialize;
use thiserror::Error;

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
    morph_targets: usize,
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
    morph_targets: usize,
    vertices: usize,
    triangles: usize,
    bytes: u64,
    skipped: usize,
    issues: Vec<IssueJson<'a>>,
}

/// The head of the character could not be shaped by its DNA, so the file lacks it.
#[derive(Debug, Error)]
#[error("the head could not be shaped by its DNA and is not in {}", .0.display())]
pub(crate) struct HeadNotShaped(PathBuf);

const fn origin_code(origin: PartOrigin) -> &'static str {
    match origin {
        PartOrigin::Body => "body",
        _ => "head",
    }
}

/// Some expressions of the head could not be made, so the file lacks them.
#[derive(Debug, Error)]
#[error("some expressions of the head could not be made and are not in {}", .0.display())]
pub(crate) struct ExpressionsNotMade(PathBuf);

fn expressions_of(args: &ChfGlbArgs) -> Expressions {
    if args.no_expressions {
        Expressions::None
    } else if !args.expressions.is_empty() {
        Expressions::Only(args.expressions.clone())
    } else if args.lod == 0 {
        Expressions::All
    } else {
        Expressions::None
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
    let head = Head::from_character(&chf.character)?.with_expressions(&expressions_of(args));
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
    } else if let Err(error) = print_report(&armour, &head, &summary, args)
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(error.into());
    }
    if armour
        .issues()
        .iter()
        .any(|issue| matches!(issue.kind(), IssueKind::ShapeFailed { .. }))
    {
        return Err(HeadNotShaped(args.out.clone()).into());
    }
    if armour.issues().iter().any(|issue| {
        matches!(
            issue.kind(),
            IssueKind::MorphsFailed { .. }
                | IssueKind::UnknownExpression { .. }
                | IssueKind::EmptyExpression { .. }
        )
    }) {
        return Err(ExpressionsNotMade(args.out.clone()).into());
    }
    Ok(())
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
                morph_targets: part.morphs().len(),
            })
            .collect(),
        nodes: summary.nodes,
        bones: summary.bones,
        meshes: summary.meshes,
        skinned_meshes: summary.skinned_meshes,
        primitives: summary.primitives,
        materials: summary.materials,
        morph_targets: summary.morph_targets,
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
        "wrote {} with the head of {} on the {} body: {} parts, {} bones, {} nodes, {} meshes ({} skinned), {} primitives, {} materials, {} morph targets, {} vertices, {} triangles, {} bytes",
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
        summary.morph_targets,
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
