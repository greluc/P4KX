// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The head of a CHF character: the body variant and the head skin its DNA names.

use std::error::Error;

use p4kx_assembly::character::{Character, Chf};
use p4kx_assembly::{BodyVariant, Head, HeadError};
use p4kx_testkit::{FEMALE_BODY_TYPE, FEMALE_RIG_HASH, MALE_BODY_TYPE, MALE_RIG_HASH, TestChf};

use crate::assembly_support::TestResult;

fn character_with(hashes: Option<(u32, u32)>) -> Result<Character, Box<dyn Error>> {
    let body_type = match hashes {
        Some((FEMALE_RIG_HASH, _)) => FEMALE_BODY_TYPE,
        _ => MALE_BODY_TYPE,
    };
    let chf = TestChf {
        body_type,
        dna_hashes: hashes,
        face_parts: (0..13).map(|_| vec![(0, 0), (0, 0)]).collect(),
        ..TestChf::v8()
    };
    Ok(Chf::read(&chf.to_bytes())?.character)
}

#[test]
fn the_rig_of_the_dna_names_the_body_variant_and_the_head_skin() -> TestResult {
    for (hash, variant, code) in [
        (MALE_RIG_HASH, BodyVariant::Male, "male"),
        (FEMALE_RIG_HASH, BodyVariant::Female, "female"),
    ] {
        let head = Head::from_character(&character_with(Some((hash, 0)))?)?;

        assert_eq!(head.variant(), variant);
        assert_eq!(head.rig(), format!("protos_human_{code}_face_t1_pu"));
        assert_eq!(head.pieces().len(), 1);
        assert_eq!(head.pieces()[0].name(), "head");
    }
    Ok(())
}

#[test]
fn a_character_without_face_dna_or_for_an_unknown_rig_has_no_head() -> TestResult {
    assert_eq!(
        Head::from_character(&character_with(None)?),
        Err(HeadError::NoFace)
    );
    assert_eq!(
        Head::from_character(&character_with(Some((0xdead_beef, 0)))?),
        Err(HeadError::UnknownRig {
            gender_hash: 0xdead_beef
        })
    );
    Ok(())
}
