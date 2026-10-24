// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The default characters of a real Archive as one GLB with textured materials, morph targets and
//! a facial animation, and the bodies of both variants, which need no `.chrm` file.

use std::error::Error;
use std::io::Read;
use std::path::Path;
use std::time::Instant;

use p4kx_assembly::character::Chf;
use p4kx_assembly::{
    ArchiveModels, Armour, ArmourPart, AssemblyOptions, BodyVariant, Expressions, ForgeItems, Head,
    LoadoutResolver, PartOrigin,
};
use p4kx_extraction::{AnimationChoice, Archive, DataForgeEntry};
use p4kx_gltf::{GlbOptions, GlbSummary, MaterialMode};
use p4kx_testkit::gltf::animation::Property;
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const DEFAULTS: [(&str, &str); 2] = [
    ("masculinedefault.chf", "facerom_t0_face"),
    ("femininedefault.chf", "ai_civ_neutral_idle_08_face"),
];

fn shipped(archive: &Archive, name: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let entry = archive
        .entries()
        .iter()
        .find(|entry| {
            entry
                .path()
                .to_ascii_lowercase()
                .replace('\\', "/")
                .ends_with(&format!("/{name}"))
        })
        .ok_or_else(|| format!("the Archive ships no {name}"))?;
    let mut bytes = Vec::new();
    archive.open_entry(entry)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the counts, sizes and times are the measurement"
)]
fn a_default_head_exports_with_materials_morph_targets_and_an_animation() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let models = ArchiveModels::new(&archive);
    let textures = models.textures();
    let mut resolver = LoadoutResolver::new(&forge);
    let mut items = ForgeItems::new(&mut resolver);
    for (name, face) in DEFAULTS {
        let started = Instant::now();
        let chf = Chf::read(&shipped(&archive, name)?)?;
        let head = Head::dressed(&chf.character, &mut items)?.with_expressions(&Expressions::All);
        let variant = head.variant();
        let armour = Armour::build(variant, head.pieces(), &models, &AssemblyOptions::default())?;
        let skipped: Vec<_> = armour
            .issues()
            .iter()
            .filter(|issue| issue.kind().skips())
            .map(|issue| format!("{}: {}", issue.name(), issue.kind()))
            .collect();
        assert!(skipped.is_empty(), "{name}: {skipped:?}");

        let set = models.animations(
            &variant.animation_folders(),
            &AnimationChoice::Named(vec![face.to_owned()]),
            &[],
        )?;
        let animations = armour.animate(&set.animations, &models);
        assert!(animations.issues().is_empty(), "{:?}", animations.issues());

        let options = GlbOptions::default()
            .with_materials(MaterialMode::Textures)
            .with_texture_mip(2);
        let mut bytes = Vec::new();
        let summary =
            armour.write_glb_animated_with(&animations, &options, &textures, &mut bytes)?;
        let elapsed = started.elapsed();
        println!("{name} ({variant:?}): built and written in {elapsed:?}");
        check_head_glb(name, &bytes, &summary)?;
    }
    Ok(())
}

#[expect(
    clippy::print_stdout,
    reason = "the counts and sizes are the measurement"
)]
fn check_head_glb(name: &str, bytes: &[u8], summary: &GlbSummary) -> TestResult {
    let glb = check_glb(bytes)?;
    let animation = summary.animations.first().ok_or("an animation")?;
    println!(
        "{name}: GLB {} bytes; {} materials ({} textured), {} images; {} morph targets; animation \
         {} with {} channels, {} keys, {:.2} s",
        bytes.len(),
        summary.materials,
        summary.textured_materials,
        summary.textures,
        summary.morph_targets,
        animation.name,
        animation.channels,
        animation.keys,
        animation.duration_seconds
    );
    assert!(summary.textured_materials >= 6, "{name}: {summary:?}");
    assert!(summary.textures > 4, "{name}: {} images", summary.textures);
    assert!(summary.morph_targets > 100, "{name}");
    assert_eq!(
        glb.document.animations().count(),
        summary.animations.len(),
        "{name}: animations written"
    );
    assert!(animation.channels > 20, "{name}: {}", animation.channels);
    assert!(
        animation.duration_seconds > 0.0,
        "{name}: {} s",
        animation.duration_seconds
    );
    assert!(
        glb.document.animations().any(|animation| animation
            .channels()
            .any(|channel| channel.target().property() == Property::MorphTargetWeights)),
        "{name}: no morph weight channel"
    );
    let head_mesh = glb
        .document
        .nodes()
        .find(|node| node.name() == Some("head") && node.mesh().is_some())
        .and_then(|node| node.mesh())
        .ok_or("the head mesh")?;
    assert!(
        glb.target_names(head_mesh.index())
            .is_some_and(|names| names.len() > 100),
        "{name}: the head's target names"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts are the measurement")]
fn the_body_of_each_variant_needs_no_chrm_and_exports_as_a_skinned_mesh() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let models = ArchiveModels::new(&archive);
    for variant in BodyVariant::ALL {
        let skeleton = variant.skeleton().replace('/', "\\").to_ascii_lowercase();
        let chrm = format!("{skeleton}m");
        let paths: Vec<String> = archive
            .entries()
            .iter()
            .map(|entry| entry.path().to_ascii_lowercase())
            .filter(|path| path.ends_with(&skeleton) || path.ends_with(&chrm))
            .collect();
        assert!(
            paths.iter().any(|path| path.ends_with(&skeleton)),
            "{variant:?}: {paths:?}"
        );
        assert!(
            !paths.iter().any(|path| path.ends_with(&chrm)),
            "{variant:?} has a .chrm now: {paths:?}"
        );

        let armour = Armour::build(variant, &[], &models, &AssemblyOptions::default())?;
        assert!(
            Path::new(armour.skeleton_file())
                .extension()
                .is_some_and(|extension| extension == "chr"),
            "{variant:?}"
        );
        assert!(armour.skeleton().bones.len() > 100, "{variant:?}");
        assert_eq!(armour.count(PartOrigin::Body), 1, "{variant:?}");
        assert!(armour.issues().is_empty(), "{:?}", armour.issues());
        let body: &ArmourPart = armour
            .parts()
            .iter()
            .find(|part| part.origin() == PartOrigin::Body)
            .ok_or("the body")?;
        assert!(
            body.model()
                .lods
                .iter()
                .flat_map(|lod| &lod.meshes)
                .any(|mesh| mesh.influences.is_some() && mesh.positions.len() > 1000),
            "{variant:?}: the body is not a skinned mesh"
        );

        let mut bytes = Vec::new();
        let summary = armour.write_glb(&GlbOptions::default(), &mut bytes)?;
        check_glb(&bytes)?;
        println!(
            "{variant:?}: {} bones, {} vertices, GLB {} bytes",
            summary.bones,
            summary.vertices,
            bytes.len()
        );
        assert_eq!(summary.skinned_meshes, 1, "{variant:?}");
    }
    Ok(())
}
