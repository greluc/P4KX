// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The body type of a CHF is checked against the head rig its DNA names.

use std::error::Error;

use p4kx_assembly::character::{Chf, Guid};
use p4kx_assembly::{ArchiveModels, Armour, AssemblyOptions, Head, PartOrigin};
use p4kx_extraction::Archive;
use p4kx_testkit::{
    FEMALE_BODY_TYPE, HEAD_VERTICES, MALE_BODY_TYPE, MALE_RIG_HASH, TestChf, head_entries, head_rig,
};

use crate::assembly_support::{TestResult, fixture};

fn codes_of_head_with(body_type: [u8; 16]) -> Result<Vec<String>, Box<dyn Error>> {
    let chf = TestChf {
        dna_hashes: Some((MALE_RIG_HASH, 0)),
        face_parts: (0..13).map(|_| vec![(0, 0), (0, 0)]).collect(),
        body_type,
        ..TestChf::v8()
    };
    let character = Chf::read(&chf.to_bytes())?.character;
    let head = Head::from_character(&character)?;
    let fixture = fixture(head_entries(Some(
        head_rig(HEAD_VERTICES, MALE_RIG_HASH).build(),
    )))?;
    let archive = Archive::open(&fixture.path)?;
    let models = ArchiveModels::new(&archive);
    let armour = Armour::build(
        head.variant(),
        head.pieces(),
        &models,
        &AssemblyOptions::default(),
    )?;
    assert_eq!(armour.count(PartOrigin::Piece), 1, "the head is still worn");
    Ok(armour
        .issues()
        .iter()
        .map(|issue| issue.kind().code().to_owned())
        .collect())
}

#[test]
fn the_testkit_body_types_are_the_guids_of_the_shipped_characters() -> TestResult {
    let male: Guid = "25f439d5-146b-4a61-a999-a486dfb68a49".parse()?;
    let female: Guid = "d0794a94-efb0-4cad-ad38-2558b4d3c253".parse()?;

    assert_eq!(Guid::from_stored(MALE_BODY_TYPE), male);
    assert_eq!(Guid::from_stored(FEMALE_BODY_TYPE), female);
    Ok(())
}

#[test]
fn the_body_type_of_the_rigs_character_raises_no_issue() -> TestResult {
    assert_eq!(codes_of_head_with(MALE_BODY_TYPE)?, Vec::<String>::new());
    Ok(())
}

#[test]
fn the_body_type_of_the_other_rig_is_listed_as_a_mismatch() -> TestResult {
    assert_eq!(
        codes_of_head_with(FEMALE_BODY_TYPE)?,
        ["body_type_mismatch"]
    );
    Ok(())
}

#[test]
fn a_zero_body_type_with_a_face_is_listed_as_a_mismatch() -> TestResult {
    assert_eq!(codes_of_head_with([0; 16])?, ["body_type_mismatch"]);
    Ok(())
}
