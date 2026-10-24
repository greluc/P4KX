// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::ffi::OsStr;
use std::path::Path;

use anyhow::Result;
use p4kx_assembly::character::{CHF_SIZE, Chf};
use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Armour, AssemblyOptions, BodyVariant, ForgeItems, Head,
    LoadoutResolver,
};
use p4kx_blender::{
    CharacterScene, PackageInfo, PackageSummary, check_output, write_character_package,
};
use p4kx_channels::channel_name;
use p4kx_extraction::{Archive, DataForgeEntry, GlbOptions, MaterialMode, is_same_file};

use crate::armour_loadout::{Files, Selection};
use crate::chf::{InvalidChf, read_capped};
use crate::chf_glb::{expressions_choice, failure};
use crate::cli_armour::BodyArg;
use crate::cli_character::{CharacterAction, CharacterArgs, CharacterPackageArgs};
use crate::cli_chf::ChfMaterialsArg;
use crate::entity_glb::GlbOutputIsArchive;
use crate::entity_package::build_info;
use crate::extract::cancel_on_interrupt;
use crate::model_animations::character_animations;

mod report;

/// Runs a `p4kx character` subcommand.
pub(crate) fn run(args: &CharacterArgs, as_json: bool) -> Result<()> {
    match &args.action {
        CharacterAction::Package(args) => package(args, as_json),
    }
}

fn glb_options(args: &CharacterPackageArgs) -> GlbOptions {
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

fn package(args: &CharacterPackageArgs, as_json: bool) -> Result<()> {
    if is_same_file(&args.out, &args.archive) {
        return Err(GlbOutputIsArchive(args.out.clone()).into());
    }
    check_output(&args.out)?;
    let chf = args.chf.as_deref().map(read_chf).transpose()?;
    let archive = crate::key::open(&args.archive)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = Files::open(ArchiveFiles::new(&archive), args.loadout.as_deref())?;
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let mut selection = Selection::gather(
        &mut resolver,
        &files,
        (&args.pieces, args.loadout.as_deref()),
    )?;
    let head = chf
        .as_ref()
        .map(|chf| dress(args, chf, &mut resolver))
        .transpose()?;
    let variant = head.as_ref().map_or_else(
        || BodyVariant::from(args.body.unwrap_or(BodyArg::Male)),
        Head::variant,
    );
    let mut pieces = std::mem::take(&mut selection.pieces);
    if let Some(head) = &head {
        pieces.extend(head.pieces().iter().cloned());
    }
    let models = ArchiveModels::new(&archive);
    let options = AssemblyOptions::default().with_lod(args.lod);
    let armour = Armour::build(variant, &pieces, &models, &options)?;
    let found = character_animations(
        &models,
        variant,
        (&args.animations, &args.animation_sources),
    )?;
    let animated = armour.animate(&found.animations, &models);

    let name = name_of(args, variant);
    let chf_name = args
        .chf
        .as_deref()
        .and_then(Path::file_name)
        .and_then(OsStr::to_str);
    let scene = CharacterScene::new(&name, &armour, &animated)
        .with_origin(chf_name, args.loadout.as_deref());
    let summary = store(args, &archive, &models, &scene)?;

    let shown = report::Shown {
        archive: archive.path(),
        args,
        name: &name,
        head: head.as_ref(),
        variant,
        selection: &selection,
        armour: &armour,
        found: &found,
    };
    report::print(&shown, &summary, as_json)?;
    failure(&armour, &args.out).map_or(Ok(()), Err)
}

fn store(
    args: &CharacterPackageArgs,
    archive: &Archive,
    models: &ArchiveModels<'_>,
    scene: &CharacterScene<'_>,
) -> Result<PackageSummary> {
    let info = PackageInfo::new(channel_name(archive.path()), build_info(archive));
    let cancel = cancel_on_interrupt()?;
    Ok(write_character_package(
        scene,
        &models.textures(),
        &glb_options(args),
        &info,
        &args.out,
        &|| cancel.is_cancelled(),
    )?)
}

fn read_chf(path: &Path) -> Result<Chf> {
    let bytes = read_capped(path, CHF_SIZE as u64)?;
    Ok(Chf::read(&bytes).map_err(|source| InvalidChf {
        path: path.to_owned(),
        source,
    })?)
}

fn dress(
    args: &CharacterPackageArgs,
    chf: &Chf,
    resolver: &mut LoadoutResolver<'_>,
) -> Result<Head> {
    let expressions = expressions_choice(args.lod, &args.expressions, args.no_expressions);
    Ok(
        Head::dressed(&chf.character, &mut ForgeItems::new(resolver))?
            .with_expressions(&expressions),
    )
}

fn name_of(args: &CharacterPackageArgs, variant: BodyVariant) -> String {
    args.name
        .clone()
        .or_else(|| {
            args.chf
                .as_ref()
                .and_then(|path| path.file_stem())
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| variant.code().to_owned())
}
