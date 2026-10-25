// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The zones of the body that worn clothing covers: the body's triangles in them are left out.

use std::collections::BTreeSet;
use std::error::Error;

use p4kx_assembly::{
    ArchiveModels, Armour, AssemblyOptions, BodyVariant, IssueKind, LoadoutResolver, PartOrigin,
    Piece,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{
    TestEntry, TestZoneChunk, ZONE_SHIRT, zone_entries, zone_entries_with_layer_textures,
};

use crate::assembly_support::{TestResult, fixture};

/// Returns where each zone of `variant` is: left zones at negative X, right ones at positive X,
/// feet at the ground, everything else at the height of the torso.
fn places(variant: BodyVariant) -> Vec<[f32; 2]> {
    variant
        .zone_names()
        .iter()
        .map(|zone| {
            let x = if zone.starts_with("l_") {
                -0.2
            } else if zone.starts_with("r_") {
                0.2
            } else {
                0.0
            };
            [x, if zone.contains("foot") { 0.1 } else { 1.2 }]
        })
        .collect()
}

fn wear_on(
    variant: BodyVariant,
    bodies: &[Vec<[f32; 2]>; 2],
    chunks: &[TestZoneChunk],
) -> Result<Armour, Box<dyn Error>> {
    wear_entries(variant, zone_entries([&bodies[0], &bodies[1]], chunks))
}

fn wear_entries(variant: BodyVariant, entries: Vec<TestEntry>) -> Result<Armour, Box<dyn Error>> {
    let fixture = fixture(entries)?;
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

fn wear(variant: BodyVariant, chunks: &[TestZoneChunk]) -> Result<Armour, Box<dyn Error>> {
    let bodies = BodyVariant::ALL.map(places);
    wear_on(variant, &bodies, chunks)
}

fn body(armour: &Armour) -> Result<&p4kx_model::Mesh, Box<dyn Error>> {
    armour
        .parts()
        .iter()
        .find(|part| part.origin() == PartOrigin::Body)
        .and_then(|part| part.model().lods.first())
        .and_then(|lod| lod.meshes.first())
        .ok_or_else(|| "no body".into())
}

fn drawn_nodes(armour: &Armour) -> Result<BTreeSet<u16>, Box<dyn Error>> {
    Ok(body(armour)?
        .submeshes
        .iter()
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
    let armour = wear(variant, &chunks)?;
    let drawn = drawn_nodes(&armour)?;
    assert_eq!(drawn.len(), variant.zone_names().len() - 2);
    assert!(!drawn.contains(&node_of(variant, "torso01_zone")?));
    assert!(!drawn.contains(&node_of(variant, "torso03_zone")?));
    assert!(drawn.contains(&node_of(variant, "torso02_zone")?));
    assert_eq!(hidden_zones(&armour), ["torso03_zone", "torso01_zone"]);
    Ok(())
}

#[test]
fn the_triangle_list_loses_the_triangles_of_the_covered_zones_too() -> TestResult {
    let variant = BodyVariant::Male;
    let all = body(&wear(variant, &[])?)?.indices.len();
    let chunks = [
        TestZoneChunk::new("torso01_zone", 1),
        TestZoneChunk::new("l_foot_zone", 1),
    ];
    let armour = wear(variant, &chunks)?;
    let mesh = body(&armour)?;
    assert_eq!(mesh.indices.len(), all - 6);
    let ranges: usize = mesh.submeshes.iter().map(|part| part.index_count).sum();
    assert_eq!(ranges, mesh.indices.len());
    let mut next = 0;
    for part in &mesh.submeshes {
        assert_eq!(part.first_index, next);
        next += part.index_count;
    }
    Ok(())
}

#[test]
fn the_female_body_has_the_bra_zone_the_male_one_lacks() -> TestResult {
    let chunks = [TestZoneChunk::new("underwear_top_zone", 1)];
    let female = wear(BodyVariant::Female, &chunks)?;
    let node = node_of(BodyVariant::Female, "underwear_top_zone")?;
    assert!(!drawn_nodes(&female)?.contains(&node));
    let male = wear(BodyVariant::Male, &chunks)?;
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
    let armour = wear(variant, &chunks)?;
    assert_eq!(drawn_nodes(&armour)?.len(), variant.zone_names().len());
    assert_eq!(hidden_zones(&armour), Vec::<String>::new());
    Ok(())
}

fn unknown_reason(armour: &Armour) -> Option<&str> {
    armour.issues().iter().find_map(|issue| match issue.kind() {
        IssueKind::BodyZonesUnknown(reason) => Some(reason.as_str()),
        _ => None,
    })
}

#[test]
fn a_body_that_does_not_fit_the_zone_table_is_kept_whole_and_reported() -> TestResult {
    let chunks = [TestZoneChunk::new("torso01_zone", 1)];
    let few = vec![[0.0, 1.2]; 5];
    let armour = wear_on(BodyVariant::Male, &[few.clone(), few], &chunks)?;
    assert_eq!(drawn_nodes(&armour)?.len(), 5);
    assert!(unknown_reason(&armour).is_some_and(|reason| reason.contains("5 zones")));
    Ok(())
}

#[test]
fn a_body_whose_zones_lie_on_the_wrong_side_is_kept_whole_and_reported() -> TestResult {
    let chunks = [TestZoneChunk::new("torso01_zone", 1)];
    let mut male = places(BodyVariant::Male);
    let at = node_of(BodyVariant::Male, "l_arm01_zone")?;
    male[usize::from(at)][0] = 0.3;
    let armour = wear_on(
        BodyVariant::Male,
        &[male, places(BodyVariant::Female)],
        &chunks,
    )?;
    assert_eq!(
        drawn_nodes(&armour)?.len(),
        BodyVariant::Male.zone_names().len()
    );
    assert!(unknown_reason(&armour).is_some_and(|reason| reason.contains("l_arm01_zone")));
    Ok(())
}

#[test]
fn layered_materials_without_a_diffuse_map_are_reported_for_each_part() -> TestResult {
    let armour = wear(BodyVariant::Male, &[])?;
    let reported: Vec<String> = armour
        .layer_issues()
        .iter()
        .filter(|issue| matches!(issue.kind(), IssueKind::LayersWithoutDiffuse(1)))
        .map(|issue| issue.name().to_owned())
        .collect();
    assert_eq!(reported, ["body", "TEST_ZoneShirt"]);
    assert!(
        armour
            .issues()
            .iter()
            .all(|issue| !matches!(issue.kind(), IssueKind::LayersWithoutDiffuse(_)))
    );
    Ok(())
}

#[test]
fn layered_materials_whose_layers_have_a_diffuse_map_are_not_reported() -> TestResult {
    let bodies = BodyVariant::ALL.map(places);
    let entries = zone_entries_with_layer_textures([&bodies[0], &bodies[1]], &[]);

    let armour = wear_entries(BodyVariant::Male, entries)?;

    assert!(
        armour.layer_issues().is_empty(),
        "{:?}",
        armour.layer_issues()
    );
    Ok(())
}
