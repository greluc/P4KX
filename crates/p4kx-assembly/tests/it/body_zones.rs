// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The zones of the body that worn clothing covers: the body's submeshes in them are left out.

use std::collections::BTreeSet;
use std::error::Error;

use p4kx_assembly::{
    ArchiveModels, Armour, AssemblyOptions, BodyVariant, IssueKind, LoadoutResolver, PartOrigin,
    Piece,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{TestZoneChunk, ZONE_SHIRT, zone_entries};

use crate::assembly_support::{TestResult, fixture};

fn wear(
    variant: BodyVariant,
    zones: usize,
    chunks: &[TestZoneChunk],
) -> Result<Armour, Box<dyn Error>> {
    let fixture = fixture(zone_entries(zones, chunks))?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find(ZONE_SHIRT)?;
    let pieces = [Piece::wearable(resolver.wearable(&record)?)];
    let models = ArchiveModels::new(&archive);
    Ok(Armour::build(
        variant,
        &pieces,
        &models,
        &AssemblyOptions::default(),
    )?)
}

fn drawn_nodes(armour: &Armour) -> Result<BTreeSet<u16>, Box<dyn Error>> {
    let body = armour
        .parts()
        .iter()
        .find(|part| part.origin() == PartOrigin::Body)
        .ok_or("no body")?;
    let lod = body.model().lods.first().ok_or("no level")?;
    Ok(lod
        .meshes
        .iter()
        .flat_map(|mesh| &mesh.submeshes)
        .map(|part| part.node_id)
        .collect())
}

fn node_of(variant: BodyVariant, zone: &str) -> Result<u16, Box<dyn Error>> {
    let at = variant
        .zone_names()
        .iter()
        .position(|name| *name == zone)
        .ok_or("no such zone")?;
    Ok(u16::try_from(at)?)
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

#[test]
fn a_zone_the_clothing_covers_is_left_out_of_the_male_body() -> TestResult {
    let variant = BodyVariant::Male;
    let chunks = [
        TestZoneChunk::new("torso01_zone", 1),
        TestZoneChunk::new("Torso03_Zone", 2),
    ];
    let armour = wear(variant, variant.zone_names().len(), &chunks)?;
    let drawn = drawn_nodes(&armour)?;
    assert_eq!(drawn.len(), variant.zone_names().len() - 2);
    assert!(!drawn.contains(&node_of(variant, "torso01_zone")?));
    assert!(!drawn.contains(&node_of(variant, "torso03_zone")?));
    assert!(drawn.contains(&node_of(variant, "torso02_zone")?));
    assert_eq!(hidden_zones(&armour), ["torso03_zone", "torso01_zone"]);
    Ok(())
}

#[test]
fn the_female_body_has_the_bra_zone_the_male_one_lacks() -> TestResult {
    let chunks = [TestZoneChunk::new("underwear_top_zone", 1)];
    let female = wear(
        BodyVariant::Female,
        BodyVariant::Male.zone_names().len(),
        &chunks,
    )?;
    let node = node_of(BodyVariant::Female, "underwear_top_zone")?;
    assert!(!drawn_nodes(&female)?.contains(&node));
    let male = wear(
        BodyVariant::Male,
        BodyVariant::Male.zone_names().len(),
        &chunks,
    )?;
    assert_eq!(
        drawn_nodes(&male)?.len(),
        BodyVariant::Male.zone_names().len()
    );
    assert_eq!(hidden_zones(&male), Vec::<String>::new());
    Ok(())
}

#[test]
fn a_chunk_that_lets_the_body_show_or_depends_on_a_condition_covers_nothing() -> TestResult {
    let variant = BodyVariant::Male;
    let chunks = [
        TestZoneChunk::new("l_arm04_zone", 1).showing(&[0]),
        TestZoneChunk::new("torso02_zone", 2).conditional(),
        TestZoneChunk::new("l_hand_zone", 0),
    ];
    let armour = wear(variant, variant.zone_names().len(), &chunks)?;
    assert_eq!(drawn_nodes(&armour)?.len(), variant.zone_names().len());
    assert_eq!(hidden_zones(&armour), Vec::<String>::new());
    Ok(())
}

#[test]
fn a_body_that_does_not_fit_the_zone_table_is_kept_whole_and_reported() -> TestResult {
    let chunks = [TestZoneChunk::new("torso01_zone", 1)];
    let armour = wear(BodyVariant::Male, 5, &chunks)?;
    assert_eq!(drawn_nodes(&armour)?.len(), 5);
    assert!(armour.issues().iter().any(|issue| matches!(
        issue.kind(),
        IssueKind::BodyZonesUnknown(reason) if reason.contains("5 zones")
    )));
    Ok(())
}

#[test]
fn layered_materials_without_a_diffuse_map_are_reported_for_each_part() -> TestResult {
    let variant = BodyVariant::Male;
    let armour = wear(variant, variant.zone_names().len(), &[])?;
    let reported: Vec<&str> = armour
        .issues()
        .iter()
        .filter(|issue| matches!(issue.kind(), IssueKind::LayersWithoutDiffuse(1)))
        .map(p4kx_assembly::Issue::name)
        .collect();
    assert_eq!(reported, ["body", "TEST_ZoneShirt"]);
    Ok(())
}
