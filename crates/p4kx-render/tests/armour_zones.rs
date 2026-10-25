// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The viewer's scene of an Armour has no triangles in the body zones that clothing covers.

use std::error::Error;

use p4kx_assembly::{ArchiveModels, Armour, AssemblyOptions, BodyVariant, LoadoutResolver, Piece};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_render::Scene;
use p4kx_testkit::{ArchiveBuilder, TestZoneChunk, ZONE_SHIRT, zone_entries};

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

fn triangles(chunks: &[TestZoneChunk]) -> Result<usize, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    let (male, female) = (places(BodyVariant::Male), places(BodyVariant::Female));
    zone_entries([&male, &female], chunks)
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    let archive = Archive::open(&path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find(ZONE_SHIRT)?;
    let pieces = [Piece::wearable(resolver.wearable(&record)?)];
    let models = ArchiveModels::new(&archive);
    let armour = Armour::build(
        BodyVariant::Male,
        &pieces,
        &models,
        &AssemblyOptions::default(),
    )?;
    let (scene, _) = Scene::from_armour(&armour)?;
    Ok(scene.indices().len() / 3)
}

#[test]
fn the_scene_of_the_viewer_loses_the_triangles_of_the_covered_zones() -> Result<(), Box<dyn Error>>
{
    let bare = triangles(&[])?;
    let covered = triangles(&[
        TestZoneChunk::new("torso01_zone", 1),
        TestZoneChunk::new("l_foot_zone", 1),
        TestZoneChunk::new("r_hand_zone", 1).showing(&[0]),
    ])?;
    assert_eq!(covered, bare - 2);
    Ok(())
}
