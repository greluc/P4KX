// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The landing gear of a ship become parts placed at their bone of the ship's model.

use crate::assembly_support::{TestResult, assemble, fixture};

use p4kx_assembly::{
    ArchiveModels, Assembly, AssemblyOptions, LoadoutResolver, MAX_ASSEMBLY_VERTICES, Part,
    Placement,
};
use p4kx_assembly::{ModelFailure, ModelProvider, ProvidedModel};
use p4kx_extraction::model::{MaterialStatus, Model};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{LANDING_GEAR_SHIP, landing_gear_entries};

fn names(assembly: &Assembly) -> Vec<&str> {
    assembly.parts().iter().map(Part::name).collect()
}

#[test]
fn a_gear_is_a_part_with_its_model_at_the_bone_of_the_ships_model() -> TestResult {
    let fixture = fixture(landing_gear_entries())?;
    let archive = Archive::open(&fixture.path)?;

    let assembly = assemble(&archive, LANDING_GEAR_SHIP, 0)?;

    assert_eq!(
        names(&assembly),
        [
            "Gear_Ship",
            "landing_gear/hardpoint_gear_front",
            "landing_gear/hardpoint_gear_missing",
            "landing_gear/hardpoint_gear_none",
            "hardpoint_tender/Gear_Tender",
            "landing_gear/bone_tender",
            "hardpoint_hangar/Gear_Snub",
            "landing_gear/bone_tender",
        ]
    );
    let gear = assembly
        .part_at("landing_gear/hardpoint_gear_front")
        .ok_or("the front gear")?;
    assert_eq!(gear.parent(), Some(0));
    assert!(gear.is_landing_gear(), "a gear is no Entity");
    assert!(gear.model().is_some(), "the gear model is read");
    assert_eq!(
        gear.placement(),
        &Placement::Helper("hardpoint_gear_front".to_owned())
    );
    assert_eq!(gear.world().translation(), [5.0, 0.0, 0.0]);
    assert!(!assembly.parts()[0].is_landing_gear());
    Ok(())
}

#[test]
fn a_gear_below_an_attached_entity_sits_at_the_bone_of_that_entitys_model() -> TestResult {
    let fixture = fixture(landing_gear_entries())?;
    let archive = Archive::open(&fixture.path)?;

    let assembly = assemble(&archive, LANDING_GEAR_SHIP, 0)?;

    let gear = assembly
        .part_at("hardpoint_tender/landing_gear/bone_tender")
        .ok_or("the tender's gear")?;
    assert_eq!(gear.parent(), Some(4), "below the tender");
    assert_eq!(
        gear.placement(),
        &Placement::Helper("bone_tender".to_owned())
    );
    assert_eq!(gear.world().translation(), [2.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn a_gear_whose_bone_the_model_lacks_sits_at_the_origin_and_is_reported() -> TestResult {
    let fixture = fixture(landing_gear_entries())?;
    let archive = Archive::open(&fixture.path)?;

    let assembly = assemble(&archive, LANDING_GEAR_SHIP, 0)?;

    let gear = assembly
        .part_at("landing_gear/hardpoint_gear_missing")
        .ok_or("the missing gear")?;
    assert_eq!(gear.placement(), &Placement::Origin);
    assert_eq!(gear.world().translation(), [0.0, 0.0, 0.0]);
    let codes: Vec<(&str, &str, &str)> = assembly
        .issues()
        .iter()
        .map(|issue| (issue.path(), issue.name(), issue.kind().code()))
        .collect();
    assert!(
        codes.contains(&(
            "landing_gear/hardpoint_gear_missing",
            "hardpoint_gear_missing",
            "helper_not_found"
        )),
        "{codes:?}"
    );
    assert!(
        codes.contains(&(
            "landing_gear/hardpoint_gear_none",
            "hardpoint_gear_none",
            "no_geometry"
        )),
        "{codes:?}"
    );
    Ok(())
}

fn build(stop: bool, max_parts: usize) -> Result<Assembly, Box<dyn std::error::Error>> {
    let fixture = fixture(landing_gear_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    if stop {
        resolver = resolver.stopping_at_docked_vehicles();
    }
    let record = resolver.entities().find(LANDING_GEAR_SHIP)?;
    let tree = resolver.resolve(&record)?;
    let options = AssemblyOptions::default().with_limits(max_parts, MAX_ASSEMBLY_VERTICES);
    Ok(Assembly::build(
        &tree,
        &ArchiveModels::new(&archive),
        &options,
    )?)
}

#[test]
fn a_docked_vehicle_that_is_not_expanded_gets_no_gear() -> TestResult {
    let assembly = build(true, 100)?;

    assert_eq!(
        names(&assembly),
        [
            "Gear_Ship",
            "landing_gear/hardpoint_gear_front",
            "landing_gear/hardpoint_gear_missing",
            "landing_gear/hardpoint_gear_none",
            "hardpoint_tender/Gear_Tender",
            "landing_gear/bone_tender",
            "hardpoint_hangar/Gear_Snub",
        ]
    );
    Ok(())
}

#[test]
fn gears_count_against_the_limit_of_parts() -> TestResult {
    let assembly = build(false, 2)?;

    assert_eq!(
        names(&assembly),
        ["Gear_Ship", "landing_gear/hardpoint_gear_front"]
    );
    let issue = assembly
        .issues()
        .iter()
        .find(|issue| issue.kind().code() == "too_many_parts")
        .ok_or("the limit is reported")?;
    assert_eq!(issue.name(), "hardpoint_gear_missing");
    Ok(())
}

struct Deploys<'a> {
    inner: ArchiveModels<'a>,
    posed: bool,
}

impl ModelProvider for Deploys<'_> {
    fn model(&self, path: &str, lod: u32) -> Result<ProvidedModel, ModelFailure> {
        self.inner.model(path, lod)
    }

    fn library(&self, entry: &str, name: &str) -> MaterialStatus {
        self.inner.library(entry, name)
    }

    fn deployed(&self, _path: &str, model: &Model) -> Result<Option<Model>, ModelFailure> {
        if !self.posed {
            return Ok(None);
        }
        let mut moved = model.clone();
        for mesh in moved.lods.iter_mut().flat_map(|lod| &mut lod.meshes) {
            for position in &mut mesh.positions {
                position[2] += 100.0;
            }
        }
        Ok(Some(moved))
    }
}

fn build_deployed(posed: bool) -> Result<Assembly, Box<dyn std::error::Error>> {
    let fixture = fixture(landing_gear_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find(LANDING_GEAR_SHIP)?;
    let tree = resolver.resolve(&record)?;
    let provider = Deploys {
        inner: ArchiveModels::new(&archive),
        posed,
    };
    Ok(Assembly::build(
        &tree,
        &provider,
        &AssemblyOptions::default(),
    )?)
}

fn height(part: &Part) -> Option<f32> {
    part.model()?
        .lods
        .iter()
        .flat_map(|lod| &lod.meshes)
        .flat_map(|mesh| &mesh.positions)
        .map(|position| position[2])
        .reduce(f32::max)
}

#[test]
fn a_gear_takes_the_model_its_provider_poses_and_an_entity_with_the_same_file_does_not()
-> TestResult {
    let assembly = build_deployed(true)?;

    let gear = assembly
        .part_at("landing_gear/hardpoint_gear_front")
        .ok_or("the gear")?;
    assert!(height(gear).is_some_and(|top| top > 90.0), "posed");
    assert!(
        assembly
            .issues()
            .iter()
            .all(|issue| issue.kind().code() != "gear_not_deployed"),
        "{:?}",
        assembly.issues()
    );
    Ok(())
}

#[test]
fn a_gear_without_a_deploy_clip_keeps_its_pose_and_is_noted() -> TestResult {
    let assembly = build_deployed(false)?;

    let gear = assembly
        .part_at("landing_gear/hardpoint_gear_front")
        .ok_or("the gear")?;
    assert!(height(gear).is_some_and(|top| top < 90.0), "not posed");
    let notes: Vec<&str> = assembly
        .issues()
        .iter()
        .filter(|issue| issue.kind().code() == "gear_not_deployed")
        .map(p4kx_assembly::Issue::path)
        .collect();
    assert!(
        notes.contains(&"landing_gear/hardpoint_gear_front"),
        "{notes:?}"
    );
    assert!(
        assembly
            .issues()
            .iter()
            .filter(|issue| issue.kind().code() == "gear_not_deployed")
            .all(|issue| !issue.kind().skips()),
        "a note, not a skip"
    );
    Ok(())
}
