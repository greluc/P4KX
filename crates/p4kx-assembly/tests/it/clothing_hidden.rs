// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The clothing rules of a Record beyond the zones it covers: the ports it hides
//! (`HiddenParts`) and the conditions of its chunks (`VisibilityConditions`).

use std::error::Error;

use p4kx_assembly::{
    ArchiveModels, Armour, ArmourPart, AssemblyOptions, BodyVariant, IssueKind, LoadoutResolver,
    Piece,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{TestZoneChunk, ZONE_JACKET, ZONE_SHIRT, jacketed_entries};

use crate::assembly_support::{TestResult, fixture};
use crate::body_zones::places;

const SHIRT_PORT: &str = "clothing_torso_0";
const JACKET_PORT: &str = "Clothing_Torso_1";

/// Wears the shirt at `SHIRT_PORT` and the jacket at `JACKET_PORT` on the male body.
fn wear(
    shirt: &[TestZoneChunk],
    jacket: &[TestZoneChunk],
    hides: &[&str],
) -> Result<Armour, Box<dyn Error>> {
    let [male, female] = BodyVariant::ALL.map(places);
    let fixture = fixture(jacketed_entries([&male, &female], shirt, jacket, hides))?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let shirt = resolver.entities().find(ZONE_SHIRT)?;
    let jacket = resolver.entities().find(ZONE_JACKET)?;
    let pieces = [
        Piece::wearable(resolver.wearable(&shirt)?).at_port(SHIRT_PORT),
        Piece::wearable(resolver.wearable(&jacket)?).at_port(JACKET_PORT),
    ];
    let models = ArchiveModels::new(&archive);
    Ok(Armour::build(
        BodyVariant::Male,
        &pieces,
        &models,
        &AssemblyOptions::default(),
    )?)
}

fn names(armour: &Armour) -> Vec<&str> {
    armour.parts().iter().map(ArmourPart::name).collect()
}

fn hidden_zones(armour: &Armour) -> Vec<String> {
    armour
        .issues()
        .iter()
        .find_map(|issue| match issue.kind() {
            IssueKind::BodyZonesHidden(zones) => Some(zones.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn undecided_zones(armour: &Armour) -> Vec<String> {
    armour
        .issues()
        .iter()
        .find_map(|issue| match issue.kind() {
            IssueKind::ClothingConditionsUndecided(zones) => Some(zones.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn hidden_pieces(armour: &Armour) -> Vec<(&str, &str, &str)> {
    armour
        .issues()
        .iter()
        .filter_map(|issue| match issue.kind() {
            IssueKind::PartHidden { port, by } => Some((issue.name(), port.as_str(), by.as_str())),
            _ => None,
        })
        .collect()
}

#[test]
fn a_piece_at_a_port_another_piece_hides_is_left_out_and_reported() -> TestResult {
    let shirt = [TestZoneChunk::new("torso01_zone", 1)];
    let jacket = [TestZoneChunk::new("l_arm01_zone", 2)];
    let armour = wear(&shirt, &jacket, &["Clothing_Torso_0"])?;
    assert_eq!(names(&armour), ["body", ZONE_JACKET]);
    assert_eq!(
        hidden_pieces(&armour),
        [(ZONE_SHIRT, SHIRT_PORT, ZONE_JACKET)]
    );
    Ok(())
}

#[test]
fn a_hidden_piece_covers_no_zone_of_the_body() -> TestResult {
    let shirt = [TestZoneChunk::new("torso01_zone", 1)];
    let jacket = [TestZoneChunk::new("l_arm01_zone", 2)];
    let armour = wear(&shirt, &jacket, &["Clothing_Torso_0"])?;
    assert_eq!(hidden_zones(&armour), ["l_arm01_zone"]);
    Ok(())
}

#[test]
fn pieces_stay_when_no_piece_hides_their_port() -> TestResult {
    let shirt = [TestZoneChunk::new("torso01_zone", 1)];
    let armour = wear(&shirt, &[], &["Hat_ItemPort"])?;
    assert_eq!(names(&armour), ["body", ZONE_SHIRT, ZONE_JACKET]);
    assert_eq!(hidden_pieces(&armour), Vec::<(&str, &str, &str)>::new());
    assert_eq!(hidden_zones(&armour), ["torso01_zone"]);
    Ok(())
}

#[test]
fn a_piece_does_not_hide_itself() -> TestResult {
    let armour = wear(&[], &[], &[JACKET_PORT])?;
    assert_eq!(names(&armour), ["body", ZONE_SHIRT, ZONE_JACKET]);
    Ok(())
}

#[test]
fn a_condition_that_names_no_port_and_no_tag_does_not_restrict_the_chunk() -> TestResult {
    let shirt = [TestZoneChunk::new("torso01_zone", 1).unrestricted()];
    let armour = wear(&shirt, &[], &[])?;
    assert_eq!(hidden_zones(&armour), ["torso01_zone"]);
    assert_eq!(undecided_zones(&armour), Vec::<String>::new());
    Ok(())
}

#[test]
fn a_chunk_conditional_on_a_port_or_a_tag_covers_nothing_and_is_reported() -> TestResult {
    let shirt = [
        TestZoneChunk::new("torso02_zone", 1).conditional(),
        TestZoneChunk::new("torso03_zone", 1).conditional_on_tag(),
        TestZoneChunk::new("theta_core_zone", 1).conditional(),
    ];
    let armour = wear(&shirt, &[], &[])?;
    assert_eq!(hidden_zones(&armour), Vec::<String>::new());
    assert_eq!(undecided_zones(&armour), ["torso02_zone", "torso03_zone"]);
    Ok(())
}
