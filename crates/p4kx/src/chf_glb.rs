// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use p4kx_assembly::character::{CHF_SIZE, Chf};
use p4kx_assembly::{
    ArchiveModels, Armour, AssemblyOptions, CharacterAnimations, Expressions, ForgeItems, Head,
    IssueKind, LoadoutResolver, PartOrigin,
};
use p4kx_extraction::{
    AnimationSet, ConversionError, DataForgeEntry, GlbOptions, GlbSummary, GltfError, MaterialMode,
    is_same_file, write_replacing,
};
use serde::Serialize;
use thiserror::Error;

use crate::chf::{InvalidChf, read_capped};
use crate::cli::ChfGlbArgs;
use crate::cli_chf::ChfMaterialsArg;
use crate::entity_glb::{GlbOutputIsArchive, IssueJson, print_issues, print_textures};
use crate::json;
use crate::model_animations::{WrittenJson, character_animations, write_notes};
use crate::query_table::one_line;

#[derive(Serialize)]
struct PartJson<'a> {
    name: &'a str,
    origin: &'static str,
    file: &'a str,
    morph_targets: usize,
}

#[derive(Serialize)]
struct FallbackJson<'a> {
    material: &'a str,
    role: &'static str,
    file: &'a str,
    reason: &'a str,
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
    textured_materials: usize,
    textures: usize,
    texture_pixels: u64,
    texture_fallbacks: Vec<FallbackJson<'a>>,
    default_material_ids: usize,
    vertices: usize,
    triangles: usize,
    bytes: u64,
    skipped: usize,
    animations: Vec<WrittenJson>,
    issues: Vec<IssueJson<'a>>,
}

/// Parts of the head of the character could not be shaped by its DNA, so the file lacks them.
#[derive(Debug, Error)]
#[error("{} could not be shaped by its DNA and is not in {}", .parts.join(", "), .out.display())]
pub(crate) struct HeadNotShaped {
    parts: Vec<String>,
    out: PathBuf,
}

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
    expressions_choice(args.lod, &args.expressions, args.no_expressions)
}

/// Returns the expressions the options ask for: none with `none`, those named, else all at LOD 0
/// and none at the other levels, which have no expressions.
pub(crate) fn expressions_choice(lod: u32, named: &[String], none: bool) -> Expressions {
    if none {
        Expressions::None
    } else if !named.is_empty() {
        Expressions::Only(named.to_vec())
    } else if lod == 0 {
        Expressions::All
    } else {
        Expressions::None
    }
}

fn glb_options(args: &ChfGlbArgs) -> GlbOptions {
    let mode = match args.materials {
        ChfMaterialsArg::Colours => MaterialMode::Colours,
        ChfMaterialsArg::Textures => MaterialMode::Textures,
    };
    let glb = GlbOptions::default()
        .with_lod(args.lod)
        .with_materials(mode)
        .with_texture_mip(args.texture_mip);
    if args.no_vertex_colours {
        glb.without_vertex_colours()
    } else {
        glb
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
    let archive = crate::key::open(&args.archive)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let head = Head::dressed(&chf.character, &mut ForgeItems::new(&mut resolver))?
        .with_expressions(&expressions_of(args));
    let models = ArchiveModels::new(&archive);
    let options = AssemblyOptions::default().with_lod(args.lod);
    let armour = Armour::build(head.variant(), head.pieces(), &models, &options)?;

    let glb = glb_options(args);
    let name = head.variant().code().to_owned();
    let textures = models.textures();
    let found = character_animations(
        &models,
        head.variant(),
        (&args.animations, &args.animation_sources),
    )?;
    let animated = armour.animate(&found.animations, &models);
    let summary = write_replacing(&args.out, |writer| {
        armour
            .write_glb_animated_with(&animated, &glb, &textures, writer)
            .map_err(|error| match error {
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
        let outcome = Outcome {
            armour: &armour,
            animated: &animated,
            summary: &summary,
        };
        json::print(&report(archive.path(), args, &head, &outcome))?;
    } else if let Err(error) = print_report(&armour, &head, &summary, args)
        .and_then(|()| print_animations(&summary, &found, &animated))
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(error.into());
    }
    failure(&armour, &args.out).map_or(Ok(()), Err)
}

/// Writes the notes about the animations: what was written, what was left out and what could not
/// be done.
fn print_animations(
    summary: &GlbSummary,
    found: &AnimationSet,
    animated: &CharacterAnimations<'_>,
) -> io::Result<()> {
    let mut stdout = BufWriter::new(io::stdout().lock());
    write_notes(&mut stdout, &summary.animations, found)?;
    print_issues(&mut stdout, animated.issues())?;
    stdout.flush()
}

/// Returns the error for what could not be shaped or made, if there is any.
pub(crate) fn failure(armour: &Armour, out: &Path) -> Option<anyhow::Error> {
    let parts: Vec<String> = armour
        .issues()
        .iter()
        .filter(|issue| matches!(issue.kind(), IssueKind::ShapeFailed { .. }))
        .map(|issue| issue.name().to_owned())
        .collect();
    if !parts.is_empty() {
        return Some(
            HeadNotShaped {
                parts,
                out: out.to_path_buf(),
            }
            .into(),
        );
    }
    armour
        .issues()
        .iter()
        .any(|issue| {
            matches!(
                issue.kind(),
                IssueKind::MorphsFailed { .. }
                    | IssueKind::UnknownExpression { .. }
                    | IssueKind::EmptyExpression { .. }
            )
        })
        .then(|| ExpressionsNotMade(out.to_path_buf()).into())
}

/// What was built and written.
struct Outcome<'a> {
    armour: &'a Armour,
    animated: &'a CharacterAnimations<'a>,
    summary: &'a GlbSummary,
}

fn report<'a>(
    archive: &Path,
    args: &ChfGlbArgs,
    head: &Head,
    outcome: &Outcome<'a>,
) -> GlbJson<'a> {
    let Outcome {
        armour,
        animated,
        summary,
    } = *outcome;
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
        vertices: summary.vertices,
        triangles: summary.triangles,
        bytes: summary.bytes,
        skipped: armour
            .issues()
            .iter()
            .filter(|issue| issue.kind().skips())
            .count(),
        animations: summary.animations.iter().map(WrittenJson::of).collect(),
        issues: armour
            .issues()
            .iter()
            .chain(animated.issues())
            .map(IssueJson::from)
            .collect(),
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
    print_textures(&mut stdout, summary)?;
    if summary.default_material_ids > 0 {
        writeln!(
            stdout,
            "{} material IDs are drawn with the engine's default material",
            summary.default_material_ids
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
