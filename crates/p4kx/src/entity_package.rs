// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, BufWriter, Write};

use anyhow::Result;
use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, EntityInterior, EntityNode,
    ForgePalettes, InteriorOptions, LoadoutResolver,
};
use p4kx_blender::{
    BuildInfo, Issue, PackageInfo, PackageSummary, Replacement, check_output, write_package,
};
use p4kx_channels::channel_name;
use p4kx_extraction::{Archive, Build, DataForgeEntry, GlbOptions, is_same_file};
use serde::Serialize;

use crate::cli_entity::EntityPackageArgs;
use crate::entity_glb::GlbOutputIsArchive;
use crate::extract::cancel_on_interrupt;
use crate::json;
use crate::query_table::one_line;

#[derive(Serialize)]
struct PackageJson<'a> {
    archive: String,
    entity: &'a str,
    guid: String,
    out: String,
    lod: u32,
    texture_mip: u32,
    nodes: usize,
    drawn_nodes: usize,
    containers: usize,
    objects: usize,
    lights: usize,
    meshes: usize,
    shared_meshes: usize,
    textures: usize,
    texture_pixels: u64,
    texture_failures: usize,
    materials: usize,
    palettes: usize,
    files: usize,
    bytes: u64,
    vertices: usize,
    triangles: usize,
    skipped: usize,
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    excluded: &'a [String],
    #[serde(skip_serializing_if = "Vec::is_empty")]
    replaced: Vec<ReplacedJson<'a>>,
    issues: &'a [Issue],
}

#[derive(Serialize)]
struct ReplacedJson<'a> {
    port: &'a str,
    entity: &'a str,
}

/// Runs `p4kx entity package`.
pub(crate) fn run(args: &EntityPackageArgs, as_json: bool) -> Result<()> {
    if is_same_file(&args.out, &args.archive) {
        return Err(GlbOutputIsArchive(args.out.clone()).into());
    }
    check_output(&args.out)?;
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
    let palettes = ForgePalettes::new(&forge);
    let options = AssemblyOptions::default().with_lod(args.lod);
    let assembly = Assembly::build_with_palettes(&tree, &models, &palettes, &options)?;
    let assembly = if args.interior {
        let interior_options = InteriorOptions::default().with_lod(args.lod);
        let interior = EntityInterior::build(&archive, &assembly, &tree, &interior_options)?;
        assembly.with_interior(interior)
    } else {
        assembly
    };
    let cancel = cancel_on_interrupt()?;
    let mut glb = GlbOptions::default()
        .with_lod(args.lod)
        .with_texture_mip(args.texture_mip);
    if args.no_vertex_colours {
        glb = glb.without_vertex_colours();
    }
    let info = info_of(&archive, args);
    let loader = models.textures();
    let summary = write_package(&assembly, &models, &loader, &glb, &info, &args.out, &|| {
        cancel.is_cancelled()
    })?;
    if as_json {
        json::print(&PackageJson::of(
            &json::path(archive.path()),
            &tree,
            args,
            &info,
            &summary,
        ))?;
        return Ok(());
    }
    match print_report(&summary, args, tree.class_name()) {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(error.into()),
        _ => Ok(()),
    }
}

impl<'a> PackageJson<'a> {
    fn of(
        archive: &str,
        tree: &'a EntityNode,
        args: &EntityPackageArgs,
        info: &'a PackageInfo,
        summary: &'a PackageSummary,
    ) -> Self {
        Self {
            archive: archive.to_owned(),
            entity: tree.class_name(),
            guid: tree.guid().to_string(),
            out: json::path(&args.out),
            lod: args.lod,
            texture_mip: args.texture_mip,
            nodes: summary.nodes,
            drawn_nodes: summary.drawn_nodes,
            containers: summary.containers,
            objects: summary.objects,
            lights: summary.lights,
            meshes: summary.meshes,
            shared_meshes: summary.shared_meshes,
            textures: summary.textures,
            texture_pixels: summary.texture_pixels,
            texture_failures: summary.texture_failures,
            materials: summary.materials,
            palettes: summary.palettes,
            files: summary.files,
            bytes: summary.bytes,
            vertices: summary.vertices,
            triangles: summary.triangles,
            skipped: summary.issues.iter().filter(|issue| issue.skipped).count(),
            excluded: &info.exclude,
            replaced: info
                .replace
                .iter()
                .map(|replacement| ReplacedJson {
                    port: &replacement.port,
                    entity: &replacement.entity,
                })
                .collect(),
            issues: &summary.issues,
        }
    }
}

/// Returns the Build of `archive` as a package records it.
pub(crate) fn build_info(archive: &Archive) -> Option<BuildInfo> {
    archive.build().map(|build: &Build| BuildInfo {
        branch: build.branch().to_owned(),
        version: build.version().to_owned(),
        public_patch: build.public_patch().map(str::to_owned),
        change_number: build.change_number().to_owned(),
        build_id: build.build_id().to_owned(),
    })
}

fn info_of(archive: &Archive, args: &EntityPackageArgs) -> PackageInfo {
    let build = build_info(archive);
    let replace = args
        .changes
        .replace
        .iter()
        .map(|text| {
            let (port, entity) = text.split_once('=').unwrap_or((text, ""));
            Replacement {
                port: port.trim().to_owned(),
                entity: entity.trim().to_owned(),
            }
        })
        .collect();
    PackageInfo::new(channel_name(archive.path()), build)
        .with_changes(args.changes.exclude.clone(), replace)
}

fn print_report(
    summary: &PackageSummary,
    args: &EntityPackageArgs,
    class_name: &str,
) -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    writeln!(
        out,
        "wrote {} of {}: {} nodes ({} drawn), {} meshes, {} materials, {} palettes, {} textures, {} files, {} vertices, {} triangles, {} bytes",
        args.out.display(),
        one_line(class_name),
        summary.nodes,
        summary.drawn_nodes,
        summary.meshes,
        summary.materials,
        summary.palettes,
        summary.textures,
        summary.files,
        summary.vertices,
        summary.triangles,
        summary.bytes
    )?;
    for port in &args.changes.exclude {
        writeln!(out, "excluded {}", one_line(port))?;
    }
    for text in &args.changes.replace {
        let (port, entity) = text.split_once('=').unwrap_or((text, ""));
        writeln!(out, "replaced {} by {}", one_line(port), one_line(entity))?;
    }
    if args.interior {
        writeln!(
            out,
            "Interior: {} containers, {} objects, {} lights",
            summary.containers, summary.objects, summary.lights
        )?;
    }
    if summary.shared_meshes > 0 {
        writeln!(
            out,
            "{} nodes draw a mesh another node draws; {} meshes are written once",
            summary.shared_meshes, summary.meshes
        )?;
    }
    if summary.texture_failures > 0 {
        writeln!(
            out,
            "{} textures could not be written; their materials keep the plain colours",
            summary.texture_failures
        )?;
    }
    for issue in &summary.issues {
        let what = if issue.skipped { "skipped" } else { "note" };
        let path = if issue.path.is_empty() {
            issue.name.clone()
        } else {
            format!("{} ({})", issue.path, issue.name)
        };
        writeln!(
            out,
            "{what} {}: {}",
            one_line(&path),
            one_line(&issue.reason)
        )?;
    }
    out.flush()
}
