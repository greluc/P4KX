// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The body of an NPC Entity loses the zones its worn clothing covers, by the rule of the
//! character package.

use std::collections::BTreeSet;

use p4kx_assembly::{Assembly, BodyVariant, IssueKind};
use p4kx_extraction::Archive;
use p4kx_testkit::{NPC_ZONE_CLASS, TestZoneChunk, npc_zone_entries};

use crate::assembly_support::{TestResult, assemble, fixture};

const BODY_PORT: &str = "Body_ItemPort";

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

fn wear(chunks: &[TestZoneChunk]) -> Result<Assembly, Box<dyn std::error::Error>> {
    let bodies = BodyVariant::ALL.map(places);
    let fixture = fixture(npc_zone_entries([&bodies[0], &bodies[1]], chunks))?;
    let archive = Archive::open(&fixture.path)?;
    assemble(&archive, NPC_ZONE_CLASS, 0)
}

fn drawn_nodes(assembly: &Assembly) -> Result<BTreeSet<u16>, Box<dyn std::error::Error>> {
    let mesh = assembly
        .part_at(BODY_PORT)
        .and_then(|part| part.model())
        .and_then(|model| model.lods.first())
        .and_then(|lod| lod.meshes.first())
        .ok_or("no body")?;
    Ok(mesh.submeshes.iter().map(|part| part.node_id).collect())
}

fn node_of(zone: &str) -> Result<u16, Box<dyn std::error::Error>> {
    let at = BodyVariant::Male
        .zone_names()
        .iter()
        .position(|name| *name == zone)
        .ok_or("no such zone")?;
    Ok(u16::try_from(at)?)
}

#[test]
fn the_zones_a_worn_garment_covers_are_left_out_of_the_body() -> TestResult {
    let chunks = [
        TestZoneChunk::new("torso01_zone", 1),
        TestZoneChunk::new("Torso03_Zone", 2),
    ];

    let assembly = wear(&chunks)?;

    let drawn = drawn_nodes(&assembly)?;
    assert_eq!(drawn.len(), BodyVariant::Male.zone_names().len() - 2);
    assert!(!drawn.contains(&node_of("torso01_zone")?));
    assert!(!drawn.contains(&node_of("torso03_zone")?));
    assert!(drawn.contains(&node_of("torso02_zone")?));
    let hidden: Vec<&[String]> = assembly
        .issues()
        .iter()
        .filter_map(|issue| match issue.kind() {
            IssueKind::BodyZonesHidden(zones) => Some(zones.as_slice()),
            _ => None,
        })
        .collect();
    assert_eq!(hidden, [["torso03_zone", "torso01_zone"]]);
    Ok(())
}

#[test]
fn a_garment_that_covers_nothing_leaves_the_body_whole() -> TestResult {
    let chunks = [TestZoneChunk::new("l_arm04_zone", 1).showing(&[0])];

    let assembly = wear(&chunks)?;

    assert_eq!(
        drawn_nodes(&assembly)?.len(),
        BodyVariant::Male.zone_names().len()
    );
    assert!(assembly.issues().iter().all(|issue| !matches!(
        issue.kind(),
        IssueKind::BodyZonesHidden(_) | IssueKind::BodyZonesUnknown(_)
    )));
    Ok(())
}
