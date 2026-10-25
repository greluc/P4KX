// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The zones of the body skins of a real Archive and the zones the clothing of Miles Eckhart
//! covers.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Armour, AssemblyOptions, BodyVariant, IssueKind, LoadoutResolver,
    PartOrigin,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_model::Model;
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;
type Means = BTreeMap<&'static str, (f32, f32)>;

const NPC: &str = "NPC_Archetypes-Male-Human-MissionGivers-MilesEckhart";
const LEVELS: u32 = 6;

fn open() -> Result<Archive, Box<dyn Error>> {
    Ok(Archive::open_with_key(
        real_archive_path()?,
        real_archive_key().ok(),
    )?)
}

fn body_of(armour: &Armour) -> Result<&Model, Box<dyn Error>> {
    Ok(armour
        .parts()
        .iter()
        .find(|part| part.origin() == PartOrigin::Body)
        .ok_or("no body")?
        .model())
}

fn nodes_of(model: &Model) -> BTreeSet<u16> {
    model
        .lods
        .iter()
        .flat_map(|level| &level.meshes)
        .flat_map(|mesh| &mesh.submeshes)
        .map(|part| part.node_id)
        .collect()
}

/// Returns the mean X and height of the vertices of each zone of the body skin at level 0.
fn zone_means(variant: BodyVariant, models: &ArchiveModels<'_>) -> Result<Means, Box<dyn Error>> {
    let options = AssemblyOptions::default().with_lod(0);
    let armour = Armour::build(variant, &[], models, &options)?;
    let mesh = body_of(&armour)?
        .lods
        .first()
        .and_then(|level| level.meshes.first())
        .ok_or("no mesh")?;
    let mut means = BTreeMap::new();
    for part in &mesh.submeshes {
        let zone = variant
            .zone_names()
            .get(usize::from(part.node_id))
            .ok_or("a node beyond the table")?;
        let indices = mesh
            .indices
            .get(part.first_index..part.first_index + part.index_count)
            .ok_or("indices")?;
        let (mut x, mut z) = (0.0_f32, 0.0_f32);
        for &index in indices {
            let at = mesh
                .positions
                .get(usize::try_from(index)?)
                .ok_or("a vertex beyond the mesh")?;
            x += at[0];
            z += at[2];
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "a submesh holds far fewer indices than f32 counts exactly"
        )]
        let count = indices.len() as f32;
        means.insert(*zone, (x / count, z / count));
    }
    Ok(means)
}

fn band_of(zone: &str) -> (f32, f32) {
    if zone.contains("leg") || zone.contains("foot") {
        (0.0, 1.0)
    } else if zone.contains("arm") || zone.contains("hand") || zone.contains("shoulder") {
        (1.1, 1.6)
    } else {
        (0.85, 1.6)
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_level_of_a_body_skin_numbers_exactly_the_zones_of_its_table() -> TestResult {
    let archive = open()?;
    let models = ArchiveModels::new(&archive);
    for variant in BodyVariant::ALL {
        let table: BTreeSet<u16> = (0..u16::try_from(variant.zone_names().len())?).collect();
        for lod in 0..LEVELS {
            let options = AssemblyOptions::default().with_lod(lod);
            let armour = Armour::build(variant, &[], &models, &options)?;
            let nodes = nodes_of(body_of(&armour)?);
            assert_eq!(nodes, table, "{} level {lod}", variant.code());
        }
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_zone_table_puts_each_zone_on_its_side_and_height_of_the_body() -> TestResult {
    let archive = open()?;
    let models = ArchiveModels::new(&archive);
    for variant in BodyVariant::ALL {
        let means = zone_means(variant, &models)?;
        for (zone, &(x, z)) in &means {
            let who = format!("{} {zone}: x {x:.2}, height {z:.2}", variant.code());
            match zone.split('_').next() {
                Some("l") => assert!(x < 0.0, "{who}"),
                Some("r") => assert!(x > 0.0, "{who}"),
                _ => assert!(x.abs() < 0.05, "{who}"),
            }
            let (low, high) = band_of(zone);
            assert!((low..high).contains(&z), "{who}");
        }
        for side in ["l", "r"] {
            let at = |part: &str| means.get(format!("{side}_{part}_zone").as_str());
            let arm: Vec<f32> = ["arm01", "arm02", "arm03", "arm04", "arm05", "hand"]
                .iter()
                .filter_map(|part| at(part).map(|(x, _)| x.abs()))
                .collect();
            assert!(arm.is_sorted(), "{} {side} arm {arm:?}", variant.code());
            let leg: Vec<f32> = ["leg01", "leg02", "leg03", "leg04", "foot"]
                .iter()
                .filter_map(|part| at(part).map(|(_, z)| -z))
                .collect();
            assert!(leg.is_sorted(), "{} {side} leg {leg:?}", variant.code());
        }
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_clothing_of_miles_eckhart_covers_his_torso_arms_and_legs_but_not_his_hands() -> TestResult {
    let archive = open()?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let models = ArchiveModels::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let record = resolver.entities().find(NPC)?;
    let tree = resolver.resolve(&record)?;
    let head = resolver.npc_head(&tree).ok_or("no head")?;
    let mut pieces = resolver.worn(&tree);
    pieces.extend(head.pieces().iter().cloned());
    let options = AssemblyOptions::default().with_lod(0);
    let armour = Armour::build(head.variant(), &pieces, &models, &options)?;
    let hidden: Vec<String> = armour
        .issues()
        .iter()
        .find_map(|issue| match issue.kind() {
            IssueKind::BodyZonesHidden { zones } => Some(zones.clone()),
            _ => None,
        })
        .ok_or("no body zone is hidden")?;
    let is_hidden = |zone: &str| hidden.iter().any(|name| name == zone);
    for zone in [
        "torso01_zone",
        "torso02_zone",
        "torso03_zone",
        "torso04_zone",
        "l_arm01_zone",
        "r_arm04_zone",
        "l_leg03_zone",
        "r_foot_zone",
    ] {
        assert!(is_hidden(zone), "{zone}: {hidden:?}");
    }
    for zone in ["l_hand_zone", "r_hand_zone", "l_arm05_zone", "l_leg04_zone"] {
        assert!(!is_hidden(zone), "{zone}: {hidden:?}");
    }
    let drawn = nodes_of(body_of(&armour)?);
    assert_eq!(
        drawn.len() + hidden.len(),
        head.variant().zone_names().len()
    );
    let unknown = |kind: &IssueKind| matches!(kind, IssueKind::BodyZonesUnknown { .. });
    assert!(!armour.issues().iter().any(|issue| unknown(issue.kind())));
    Ok(())
}
