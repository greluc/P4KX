// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The default characters the game ships, with the facial joints in their skeleton, a facial
//! animation evaluated through the rig logic and a body animation, each exported as one valid GLB.

use std::error::Error;
use std::io::Read;
use std::time::Instant;

use p4kx_assembly::character::Chf;
use p4kx_assembly::{
    ArchiveModels, Armour, AssemblyOptions, BodyVariant, Expressions, Head, IssueKind,
};
use p4kx_extraction::{AnimationChoice, Archive};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

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

fn built(archive: &Archive, name: &str) -> Result<(Armour, BodyVariant), Box<dyn Error>> {
    let models = ArchiveModels::new(archive);
    let chf = Chf::read(&shipped(archive, name)?)?;
    let head = Head::from_character(&chf.character)?.with_expressions(&Expressions::All);
    let variant = head.variant();
    let armour = Armour::build(variant, head.pieces(), &models, &AssemblyOptions::default())?;
    Ok((armour, variant))
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts and times are the measurement")]
fn the_facial_joints_join_the_skeleton_and_the_head_binds_to_them() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    for name in ["masculinedefault.chf", "femininedefault.chf"] {
        let started = Instant::now();
        let (armour, _) = built(&archive, name)?;
        let body = ArchiveModels::new(&archive)
            .model_bones(armour.variant())
            .unwrap_or_default();
        println!(
            "{name}: skeleton {} bones (body {body}), built in {:?}",
            armour.skeleton().bones.len(),
            started.elapsed()
        );
        assert!(armour.skeleton().bones.len() >= body + 150, "{name}");
        let codes: Vec<&str> = armour
            .issues()
            .iter()
            .map(|issue| issue.kind().code())
            .collect();
        assert!(!codes.contains(&"face_joints_failed"), "{name}: {codes:?}");
        assert!(!codes.contains(&"bones_not_in_skeleton"), "{name}: {codes:?}");
        let mut bytes = Vec::new();
        armour.write_glb(&GlbOptions::default(), &mut bytes)?;
        check_glb(&bytes)?;
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts, sizes and times are the measurement")]
fn a_facial_animation_moves_the_facial_joints_and_sets_morph_weights() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let models = ArchiveModels::new(&archive);
    for (name, face) in [
        ("masculinedefault.chf", "facerom_t0_face"),
        ("femininedefault.chf", "ai_civ_neutral_idle_08_face"),
    ] {
        let (armour, variant) = built(&archive, name)?;
        let started = Instant::now();
        let set = models.animations(
            &variant.animation_folders(),
            &AnimationChoice::Named(vec![face.to_owned()]),
            &[],
        )?;
        println!("{name}: read {} animations in {:?}", set.animations.len(), started.elapsed());
        assert!(!set.animations.is_empty());
        let started = Instant::now();
        let animations = armour.animate(&set.animations, &models);
        let evaluated = started.elapsed();
        assert!(animations.issues().is_empty(), "{:?}", animations.issues());
        let sampled = animations.sampled();
        assert_eq!(sampled.len(), set.animations.len());
        let first = sampled.first().ok_or("a facial animation")?;
        println!(
            "{face}: {} samples over {:.2} s, {} joints moved, {} weight runs, evaluated in {evaluated:?}",
            first.times.len(),
            first.times.last().copied().unwrap_or_default(),
            first.bones.len(),
            first.weights.len()
        );
        assert!(first.bones.len() > 20, "{}", first.bones.len());
        assert!(!first.weights.is_empty());

        let mut bytes = Vec::new();
        let started = Instant::now();
        let summary = armour.write_glb_animated(&animations, &GlbOptions::default(), &mut bytes)?;
        println!(
            "{face}: GLB {} bytes in {:?}, {} animations, {} channels",
            bytes.len(),
            started.elapsed(),
            summary.animations.len(),
            summary.animations.iter().map(|a| a.channels).sum::<usize>()
        );
        let glb = check_glb(&bytes)?;
        assert_eq!(glb.document.animations().count(), summary.animations.len());
        let weights = sampled
            .iter()
            .flat_map(|animation| &animation.weights)
            .flat_map(|weights| &weights.values);
        assert!(weights.clone().all(|weight| weight.is_finite() && weight.abs() < 3.0));
        assert!(weights.clone().any(|weight| *weight > 0.2));
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts, sizes and times are the measurement")]
fn a_body_animation_moves_the_bones_of_the_body() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let models = ArchiveModels::new(&archive);
    let (armour, variant) = built(&archive, "masculinedefault.chf")?;
    let started = Instant::now();
    let set = models.animations(
        &variant.animation_folders(),
        &AnimationChoice::Named(vec!["nw_neutral_stand_getupback".to_owned()]),
        &[],
    )?;
    println!("read {} animations in {:?}", set.animations.len(), started.elapsed());
    let animations = armour.animate(&set.animations, &models);
    assert!(animations.issues().is_empty(), "{:?}", animations.issues());
    let mut bytes = Vec::new();
    let summary = armour.write_glb_animated(&animations, &GlbOptions::default(), &mut bytes)?;
    let animation = summary.animations.first().ok_or("an animation")?;
    println!(
        "{}: {} channels, {} keys, {:.2} s, {} tracks skipped, GLB {} bytes",
        animation.name,
        animation.channels,
        animation.keys,
        animation.duration_seconds,
        animation.skipped_tracks.len(),
        bytes.len()
    );
    assert!(animation.channels > 100);
    check_glb(&bytes)?;
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn an_animation_no_track_of_which_fits_is_listed_and_not_written() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let models = ArchiveModels::new(&archive);
    let (armour, _) = built(&archive, "masculinedefault.chf")?;
    let set = models.animations(
        &["Animations/Characters/Creatures"],
        &AnimationChoice::All,
        &[],
    )?;
    let few = &set.animations[..set.animations.len().min(3)];
    let animations = armour.animate(few, &models);
    assert!(animations.is_empty());
    assert!(animations
        .issues()
        .iter()
        .all(|issue| matches!(issue.kind(), IssueKind::AnimationNotUsed { .. })));
    Ok(())
}
