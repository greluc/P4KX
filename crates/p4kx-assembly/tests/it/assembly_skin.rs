// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A part on a skin port is drawn in the body's bind pose, whatever its helper and offset say.

use crate::assembly_support::{TestResult, assemble, fixture};

use p4kx_assembly::{Assembly, Placement};
use p4kx_extraction::Archive;
use p4kx_testkit::{SKIN_BODY, skin_attachment_entries};

fn skin_body() -> Result<Assembly, Box<dyn std::error::Error>> {
    let fixture = fixture(skin_attachment_entries())?;
    let archive = Archive::open(&fixture.path)?;
    assemble(&archive, SKIN_BODY, 0)
}

#[test]
fn a_skin_ignores_its_bone_helper_and_offset() -> TestResult {
    let assembly = skin_body()?;

    let skin = assembly.part_at("legs").ok_or("the skin")?;
    assert_eq!(skin.placement(), &Placement::Skin);
    assert_eq!(skin.world().translation(), [0.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn a_skin_gets_no_helper_note_even_when_the_model_lacks_the_helper() -> TestResult {
    let assembly = skin_body()?;

    let gone = assembly.part_at("gone").ok_or("the skin")?;
    assert_eq!(gone.placement(), &Placement::Skin);
    assert!(
        assembly
            .issues()
            .iter()
            .all(|issue| issue.kind().code() != "helper_not_found"),
        "{:?}",
        assembly.issues()
    );
    Ok(())
}

#[test]
fn a_skin_without_a_helper_node_is_still_a_skin() -> TestResult {
    let assembly = skin_body()?;

    let bare = assembly.part_at("bare").ok_or("the skin")?;
    assert_eq!(bare.placement(), &Placement::Skin);
    assert_eq!(bare.world().translation(), [0.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn a_bone_attachment_keeps_its_helper_and_offset() -> TestResult {
    let assembly = skin_body()?;

    let hand = assembly.part_at("hand").ok_or("the bone part")?;
    assert_eq!(hand.placement(), &Placement::Helper("Hips".to_owned()));
    assert_eq!(hand.world().translation(), [7.0, 1.0, 0.0]);
    Ok(())
}
