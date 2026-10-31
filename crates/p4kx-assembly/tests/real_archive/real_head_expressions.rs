// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The default characters the game ships, as heads with their expression blendshapes written as
//! morph targets, exported as one valid GLB.

use std::error::Error;
use std::io::Read;
use std::time::Instant;

use p4kx_assembly::character::Chf;
use p4kx_assembly::{
    ArchiveModels, Armour, ArmourPart, AssemblyOptions, BodyVariant, Expressions, Head,
};
use p4kx_extraction::Archive;
use p4kx_gltf::GlbOptions;
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const DEFAULTS: [(&str, BodyVariant); 2] = [
    ("masculinedefault.chf", BodyVariant::Male),
    ("femininedefault.chf", BodyVariant::Female),
];

fn shipped(archive: &Archive, name: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let entry = archive
        .entries()
        .iter()
        .find(|entry| entry.path().to_ascii_lowercase().ends_with(name))
        .ok_or_else(|| format!("the Archive ships no {name}"))?;
    let mut bytes = Vec::new();
    archive.open_entry(entry)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn head_part(armour: &Armour) -> Result<&ArmourPart, Box<dyn Error>> {
    Ok(armour
        .parts()
        .iter()
        .find(|part| part.name() == "head")
        .ok_or("the head")?)
}

#[expect(
    clippy::print_stdout,
    reason = "the target count, the size and the times are the measurement"
)]
fn check_targets(name: &str, part: &ArmourPart) {
    let morphs = part.morphs();
    println!("{name}: {} morph targets", morphs.len());
    assert!(morphs.len() > 100, "{name}: {}", morphs.len());
    let vertices = part
        .model()
        .lods
        .first()
        .and_then(|lod| lod.meshes.first())
        .map_or(0, |mesh| mesh.positions.len());
    for morph in morphs {
        assert!(!morph.name.is_empty(), "{name}");
        assert!(!morph.vertices.is_empty(), "{}", morph.name);
        let largest = morph
            .deltas
            .iter()
            .flatten()
            .fold(0.0_f32, |largest, value| largest.max(value.abs()));
        assert!(largest < 0.3, "{}: {largest} m", morph.name);
        assert!(
            morph
                .vertices
                .last()
                .is_some_and(|&last| (last as usize) < vertices),
            "{}",
            morph.name
        );
    }
}

#[expect(
    clippy::print_stdout,
    reason = "the target count, the size and the times are the measurement"
)]
fn check_glb_of(name: &str, armour: &Armour, lod: u32) -> TestResult {
    let started = Instant::now();
    let mut bytes = Vec::new();
    let summary = armour.write_glb(&GlbOptions::default().with_lod(lod), &mut bytes)?;
    let written = started.elapsed();
    let glb = check_glb(&bytes)?;
    println!(
        "{name} LOD{lod}: GLB {} bytes in {written:?}, {} targets, {} vertices",
        bytes.len(),
        summary.morph_targets,
        summary.vertices,
    );
    let morphs = head_part(armour)?.morphs();
    assert_eq!(summary.morph_targets, morphs.len(), "{name} LOD{lod}");
    let mesh = glb
        .document
        .nodes()
        .find(|node| node.name() == Some("head") && node.mesh().is_some())
        .and_then(|node| node.mesh())
        .ok_or("the head mesh")?;
    let names = glb.target_names(mesh.index()).ok_or("target names")?;
    assert_eq!(names.len(), morphs.len(), "{name}: the target names");
    assert!(
        names.iter().all(|name| !name.is_empty()),
        "{name}: an empty target name"
    );
    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), names.len(), "{name}: target names repeat");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_default_characters_get_their_expressions_as_named_morph_targets() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let models = ArchiveModels::new(&archive);
    for (name, variant) in DEFAULTS {
        let chf = Chf::read(&shipped(&archive, name)?)?;
        let head = Head::from_character(&chf.character)?.with_expressions(&Expressions::All);
        assert_eq!(head.variant(), variant, "{name}");
        let started = Instant::now();
        let armour = Armour::build(variant, head.pieces(), &models, &AssemblyOptions::default())?;
        let built = started.elapsed();

        let skipped: Vec<_> = armour
            .issues()
            .iter()
            .filter(|i| i.kind().skips())
            .collect();
        assert!(skipped.is_empty(), "{name}: {skipped:?}");
        assert!(built.as_secs() < 30, "{name}: {built:?}");
        check_targets(name, head_part(&armour)?);
        check_glb_of(name, &armour, 0)?;
    }
    Ok(())
}

/// The most a GLB of a lower level may grow by its expressions, as a multiple of the same GLB
/// without them.
const MAX_GROWTH: f64 = 5.0;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the sizes and times are the measurement"
)]
fn the_expressions_of_a_lower_level_stay_a_few_times_the_mesh_and_none_means_none() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let models = ArchiveModels::new(&archive);
    let chf = Chf::read(&shipped(&archive, "masculinedefault.chf")?)?;
    let sized = |expressions: &Expressions, lod: u32| -> Result<(usize, usize), Box<dyn Error>> {
        let head = Head::from_character(&chf.character)?.with_expressions(expressions);
        let started = Instant::now();
        let armour = Armour::build(
            BodyVariant::Male,
            head.pieces(),
            &models,
            &AssemblyOptions::default().with_lod(lod),
        )?;
        println!("LOD{lod} {expressions:?}: built in {:?}", started.elapsed());
        let mut bytes = Vec::new();
        armour.write_glb(&GlbOptions::default().with_lod(lod), &mut bytes)?;
        Ok((bytes.len(), head_part(&armour)?.morphs().len()))
    };
    for lod in 0..=3 {
        let (without, none) = sized(&Expressions::None, lod)?;
        let (with, all) = sized(&Expressions::All, lod)?;
        #[expect(clippy::cast_precision_loss, reason = "a ratio of two file sizes")]
        let growth = with as f64 / without as f64;
        println!("LOD{lod}: {without} bytes without, {with} with ({growth:.2}x)");
        assert_eq!(none, 0, "LOD{lod}: --no-expressions writes no targets");
        assert!(all > 100, "LOD{lod}");
        assert!(growth < MAX_GROWTH, "LOD{lod}: {growth}");
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts are the measurement")]
fn the_lower_levels_get_the_expressions_of_level_zero_carried_over() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let models = ArchiveModels::new(&archive);
    let chf = Chf::read(&shipped(&archive, "masculinedefault.chf")?)?;
    let head = Head::from_character(&chf.character)?.with_expressions(&Expressions::All);
    let build = |lod: u32| {
        Armour::build(
            BodyVariant::Male,
            head.pieces(),
            &models,
            &AssemblyOptions::default().with_lod(lod),
        )
    };
    let peak = |armour: &Armour| -> Result<f32, Box<dyn Error>> {
        Ok(head_part(armour)?
            .morphs()
            .iter()
            .flat_map(|morph| morph.deltas.iter().flatten())
            .fold(0.0_f32, |peak, value| peak.max(value.abs())))
    };
    let base = build(0)?;
    let (base_targets, base_peak) = (head_part(&base)?.morphs().len(), peak(&base)?);

    for level in 1..=3 {
        let armour = build(level)?;
        let part = head_part(&armour)?;
        println!(
            "LOD{level}: {} of {base_targets} targets, peak {} m against {base_peak} m",
            part.morphs().len(),
            peak(&armour)?
        );
        check_targets(&format!("LOD{level}"), part);
        assert!(part.morphs().len() <= base_targets, "LOD{level}");
        let ratio = peak(&armour)? / base_peak;
        assert!((0.5..=1.5).contains(&ratio), "LOD{level}: {ratio}");
        let codes: Vec<&str> = armour
            .issues()
            .iter()
            .map(|issue| issue.kind().code())
            .collect();
        assert!(!codes.contains(&"morphs_failed"), "LOD{level}: {codes:?}");
        assert!(
            !armour.issues().iter().any(|issue| issue.kind().skips()),
            "{:?}",
            armour.issues()
        );
        check_glb_of("masculinedefault", &armour, level)?;
    }
    Ok(())
}
