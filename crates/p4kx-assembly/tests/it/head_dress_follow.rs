// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The eyes and teeth of a head are shaped by their own rigs, and the hair follows the
//! expressions of the head.

use std::error::Error;

use p4kx_assembly::{
    ArchiveModels, Armour, ArmourPart, AssemblyOptions, BodyVariant, Expressions, Piece,
};
use p4kx_chunks::ChunkFile;
use p4kx_extraction::Archive;
use p4kx_gltf::GlbOptions;
use p4kx_model::Animation;
use p4kx_testkit::{
    HEAD_VERTICES, MALE_HEAD_FOLDER, MALE_RIG, MALE_RIG_HASH, TestAnimation, TestEntry, TestTrack,
    check_glb, dressed_entries, head_entries, head_rig,
};

use crate::assembly_support::{TestResult, fixture};
use crate::head_build::{dna_of_head_two_in_region_three, head_path};

const HAIR: &str = "Objects/Characters/Human/heads/shared/hair_v2/sc/male/m_hair_test.skin";

fn refusal(entries: Vec<TestEntry>, part: &str, rig: &str) -> TestResult {
    let fixture = fixture(entries)?;
    let archive = Archive::open(&fixture.path)?;
    let piece = Piece::file(
        part,
        format!("{MALE_HEAD_FOLDER}/{MALE_RIG}_{rig}.skin"),
        None,
    )
    .shaped(dna_of_head_two_in_region_three());

    let armour = Armour::build(
        BodyVariant::Male,
        &[piece],
        &ArchiveModels::new(&archive),
        &AssemblyOptions::default(),
    )?;

    assert_eq!(armour.parts().len(), 1, "only the body is left");
    let issue = armour.issues().first().ok_or("an issue")?;
    assert_eq!(issue.kind().code(), "shape_failed", "the kind");
    assert_eq!(issue.name(), part, "the part named");
    assert!(issue.kind().skips(), "the part is skipped");
    let text = issue.kind().to_string();
    assert!(
        text.contains("covers 6 vertices, the skin has 24"),
        "{text}"
    );
    Ok(())
}

#[test]
fn an_eyes_rig_that_covers_fewer_vertices_than_the_eyes_use_is_refused() -> TestResult {
    refusal(
        dressed_entries(Some(head_rig(6, MALE_RIG_HASH).build())),
        "eyes_itemport",
        "eyes",
    )
}

#[test]
fn a_teeth_rig_that_covers_fewer_vertices_than_the_teeth_use_is_refused() -> TestResult {
    refusal(
        head_entries(Some(head_rig(6, MALE_RIG_HASH).build())),
        "teeth",
        "teeth",
    )
}

fn face_animation() -> Result<Animation, Box<dyn Error>> {
    let source = TestAnimation::new("blink.caf", 30).track(
        TestTrack::new(crc32fast::hash(b"CTL_a")).positions(&[
            (0, [0.0, 0.0, 0.0]),
            (10, [1.0, 0.0, 0.0]),
            (20, [0.0, 0.0, 0.0]),
        ]),
    );
    let bytes = source.caf();
    Ok(Animation::read_caf(
        source.name(),
        &ChunkFile::parse(bytes.as_slice())?,
    )?)
}

fn named<'a>(armour: &'a Armour, name: &str) -> Result<&'a ArmourPart, Box<dyn Error>> {
    Ok(armour
        .parts()
        .iter()
        .find(|part| part.name() == name)
        .ok_or_else(|| format!("no part {name}"))?)
}

fn index_of(armour: &Armour, name: &str) -> Result<usize, Box<dyn Error>> {
    Ok(armour
        .parts()
        .iter()
        .position(|part| part.name() == name)
        .ok_or_else(|| format!("no part {name}"))?)
}

struct Worn {
    armour: Armour,
    archive: Archive,
    _fixture: crate::assembly_support::Fixture,
}

fn worn(expressions: Expressions) -> Result<Worn, Box<dyn Error>> {
    let fixture = fixture(dressed_entries(Some(
        head_rig(HEAD_VERTICES, MALE_RIG_HASH).build(),
    )))?;
    let archive = Archive::open(&fixture.path)?;
    let head = Piece::file("head", head_path(), None)
        .shaped(dna_of_head_two_in_region_three())
        .with_expressions(expressions);
    let hair = Piece::file("hair", HAIR, None).following_head();
    let armour = Armour::build(
        BodyVariant::Male,
        &[head, hair],
        &ArchiveModels::new(&archive),
        &AssemblyOptions::default(),
    )?;
    Ok(Worn {
        armour,
        archive,
        _fixture: fixture,
    })
}

#[test]
fn the_hair_gets_a_morph_target_for_each_expression_target_that_moves_it() -> TestResult {
    let worn = worn(Expressions::All)?;

    let head = named(&worn.armour, "head")?;
    let hair = named(&worn.armour, "hair")?;

    let names = |part: &ArmourPart| -> Vec<String> {
        part.morphs()
            .iter()
            .map(|target| target.name.clone())
            .collect()
    };
    assert_eq!(names(head), ["CTL_a", "CTL_b"]);
    assert_eq!(names(hair), names(head), "the same expressions, in order");
    for target in hair.morphs() {
        assert!(!target.vertices.is_empty(), "{}", target.name);
        assert!(target.vertices.is_sorted_by(|a, b| a < b));
        assert!(
            target
                .deltas
                .iter()
                .flatten()
                .all(|value| value.is_finite()),
            "{}",
            target.name
        );
    }
    assert!(
        worn.armour.issues().is_empty(),
        "{:?}",
        worn.armour.issues()
    );
    Ok(())
}

#[test]
fn without_expressions_the_hair_has_no_morph_targets() -> TestResult {
    let worn = worn(Expressions::None)?;

    assert_eq!(named(&worn.armour, "hair")?.morphs(), []);
    Ok(())
}

#[test]
fn the_morphed_hair_is_written_to_a_valid_glb() -> TestResult {
    let worn = worn(Expressions::All)?;
    let mut bytes = Vec::new();

    let summary = worn.armour.write_glb(&GlbOptions::default(), &mut bytes)?;

    check_glb(&bytes)?;
    assert_eq!(summary.morph_targets, 4, "two on the head, two on the hair");
    Ok(())
}

#[test]
fn a_facial_animation_sets_the_weights_of_the_hair_like_those_of_the_head() -> TestResult {
    let worn = worn(Expressions::All)?;
    let models = ArchiveModels::new(&worn.archive);
    let list = [face_animation()?];

    let animations = worn.armour.animate(&list, &models);

    assert!(animations.issues().is_empty(), "{:?}", animations.issues());
    let [face] = animations.sampled() else {
        return Err("one facial animation".into());
    };
    let of = |name: &str| -> Result<&Vec<f32>, Box<dyn Error>> {
        let part = index_of(&worn.armour, name)?;
        Ok(&face
            .weights
            .iter()
            .find(|run| run.piece == part)
            .ok_or_else(|| format!("weights of {name}"))?
            .values)
    };
    assert_eq!(of("head")?, &[0.0, 0.0, 1.0, 0.0, 0.0, 0.0]);
    assert_eq!(of("hair")?, of("head")?);

    let mut bytes = Vec::new();
    worn.armour
        .write_glb_animated(&animations, &GlbOptions::default(), &mut bytes)?;
    check_glb(&bytes)?;
    Ok(())
}
