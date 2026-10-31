// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `Piece::body`: the choice of a Character for its body styles the default body and adds no
//! part, whatever else is worn.

use std::error::Error;

use p4kx_assembly::character::BODY_COLOR;
use p4kx_assembly::{
    ArchiveModels, Armour, AssemblyOptions, BodyVariant, IssueKind, LoadoutResolver, Piece,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{
    DRESSED_HEAD_MATERIAL, DRESSED_UNKNOWN_MATERIAL, TestEntry, TestMtl, TestMtlMaterial,
    TestZoneChunk, ZONE_JACKET, ZONE_SHIRT, jacketed_entries,
};

use crate::assembly_support::{TestResult, fixture};
use crate::body_zones::places;
use crate::head_dress::{
    archive_entries, character_with, colour, dress_character, material, named, near,
};

const SKIN_LIBRARY: &str = "Objects/test/skin.mtl";
const EYE_LIBRARY: &str = "Objects/test/eye.mtl";

fn library(path: &str, shader: &str) -> TestEntry {
    let skin = TestMtlMaterial::new("caps").shader(shader);
    TestEntry::stored(
        format!("Data\\{}", path.replace('/', "\\")),
        TestMtl::multi(&[skin]).cryxml(),
    )
}

/// Wears the shirt at its port, the jacket hiding `hides`, and `extra` pieces before them.
fn wear(
    extra: Vec<Piece>,
    shirt: &[TestZoneChunk],
    hides: &[&str],
) -> Result<Armour, Box<dyn Error>> {
    let [male, female] = BodyVariant::ALL.map(places);
    let mut entries = jacketed_entries(
        [&male, &female],
        (shirt, &[]),
        &[TestZoneChunk::new("l_arm01_zone", 2)],
        hides,
    );
    entries.push(library(SKIN_LIBRARY, "HumanSkin_V2"));
    entries.push(library(EYE_LIBRARY, "Eye"));
    let fixture = fixture(entries)?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let shirt = resolver.wearable(&resolver.entities().find(ZONE_SHIRT)?)?;
    let jacket = resolver.wearable(&resolver.entities().find(ZONE_JACKET)?)?;
    let mut pieces = extra;
    pieces.push(Piece::wearable(shirt).at_port("clothing_torso_0"));
    pieces.push(Piece::wearable(jacket).at_port("Clothing_Torso_1"));
    Ok(Armour::build(
        BodyVariant::Male,
        &pieces,
        &ArchiveModels::new(&archive),
        &AssemblyOptions::default(),
    )?)
}

fn body_shader(armour: &Armour) -> Result<String, Box<dyn Error>> {
    let part = armour
        .parts()
        .iter()
        .find(|part| part.name() == "body")
        .ok_or("no body part")?;
    let library = part
        .materials()
        .library()
        .ok_or("the body has no library")?;
    Ok(library
        .materials
        .iter()
        .map(|material| material.shader.clone())
        .find(|shader| !shader.is_empty())
        .unwrap_or_default())
}

fn is_hidden(armour: &Armour, name: &str) -> bool {
    armour
        .issues()
        .iter()
        .any(|issue| issue.name() == name && matches!(issue.kind(), IssueKind::PartHidden { .. }))
}

#[test]
fn the_body_piece_restyles_the_body_next_to_a_piece_that_hides_the_body_and_head_ports()
-> TestResult {
    let restyle = Piece::body().with_material(SKIN_LIBRARY);
    let armour = wear(
        vec![restyle],
        &[TestZoneChunk::new("torso01_zone", 1)],
        &["head_itemport", "body_itemport"],
    )?;

    assert_eq!(body_shader(&armour)?, "HumanSkin_V2");
    assert!(
        !is_hidden(&armour, "body"),
        "the body piece is never hidden"
    );
    Ok(())
}

#[test]
fn the_body_piece_restyles_the_body_that_a_covering_piece_cuts() -> TestResult {
    let restyle = Piece::body().with_material(SKIN_LIBRARY);
    let armour = wear(vec![restyle], &[TestZoneChunk::new("torso01_zone", 1)], &[])?;

    assert_eq!(body_shader(&armour)?, "HumanSkin_V2");
    assert!(
        armour
            .issues()
            .iter()
            .any(|issue| matches!(issue.kind(), IssueKind::BodyZonesHidden(_))),
        "the shirt still covers its zone"
    );
    Ok(())
}

#[test]
fn only_the_first_body_piece_restyles_the_body() -> TestResult {
    let first = Piece::body().with_material(SKIN_LIBRARY);
    let second = Piece::body().with_material(EYE_LIBRARY);
    let armour = wear(vec![first, second], &[], &[])?;

    assert_eq!(body_shader(&armour)?, "HumanSkin_V2");
    Ok(())
}

#[test]
fn a_base_material_of_the_body_that_no_lookup_table_lists_is_reported() -> TestResult {
    let unknown = material(
        "body_itemport",
        DRESSED_UNKNOWN_MATERIAL,
        "shader_head",
        Vec::new(),
        vec![colour(BODY_COLOR, [0, 0, 255, 255])],
        Vec::new(),
    )?;
    let dressed = dress_character(archive_entries(), &character_with(vec![unknown])?)?;

    assert!(
        dressed.armour.issues().iter().any(|issue| {
            issue.name() == "body" && issue.kind().code() == "material_choice_unknown"
        }),
        "{:?}",
        dressed.armour.issues()
    );
    let body = named(&dressed, "body")?;
    let library = body
        .materials()
        .library()
        .ok_or("the body has no library")?;
    assert!(
        library.materials.iter().all(|skin| near(
            [skin.diffuse[0], skin.diffuse[1], skin.diffuse[2], 1.0],
            [0.5; 3]
        )),
        "the default library stays: {:?}",
        library.materials
    );
    Ok(())
}

#[test]
fn the_body_takes_the_base_material_and_skin_tone_the_character_chose_for_it() -> TestResult {
    let body_choice = material(
        "body_itemport",
        DRESSED_HEAD_MATERIAL,
        "shader_head",
        Vec::new(),
        vec![colour(BODY_COLOR, [0, 0, 255, 255])],
        Vec::new(),
    )?;
    let dressed = dress_character(archive_entries(), &character_with(vec![body_choice])?)?;

    let body = named(&dressed, "body")?;
    let library = body
        .materials()
        .library()
        .ok_or("the body has no library")?;
    assert!(
        library.materials.iter().all(|skin| near(
            [skin.diffuse[0], skin.diffuse[1], skin.diffuse[2], 1.0],
            [0.0, 0.0, 1.0]
        )),
        "every body skin takes the skin tone: {:?}",
        library.materials
    );
    assert_eq!(
        dressed.armour.count(p4kx_assembly::PartOrigin::Body),
        1,
        "no second body"
    );
    assert_eq!(
        dressed.armour.count(p4kx_assembly::PartOrigin::Piece),
        5,
        "no part of its own"
    );
    Ok(())
}

#[test]
fn a_character_that_chose_nothing_for_its_body_keeps_the_default_body() -> TestResult {
    let dressed = dress_character(archive_entries(), &character_with(Vec::new())?)?;

    let body = named(&dressed, "body")?;
    let library = body
        .materials()
        .library()
        .ok_or("the body has no library")?;
    assert!(
        library.materials.iter().all(|skin| near(
            [skin.diffuse[0], skin.diffuse[1], skin.diffuse[2], 1.0],
            [0.5; 3]
        )),
        "{:?}",
        library.materials
    );
    Ok(())
}
