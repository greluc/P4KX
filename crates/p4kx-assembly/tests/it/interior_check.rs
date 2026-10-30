// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The check of an Interior's containers against the hardpoints of its Assembly, on a synthetic
//! ship.

use p4kx_assembly::{
    ArchiveModels, Assembly, AssemblyOptions, EntityInterior, InteriorFit, InteriorOptions,
    LoadoutResolver,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{ENTITY_INTERIOR_CLASS, entity_interior_entries};

use crate::assembly_support::{TestResult, fixture};

fn measured() -> Result<InteriorFit, Box<dyn std::error::Error>> {
    let fixture = fixture(entity_interior_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find(ENTITY_INTERIOR_CLASS)?;
    let tree = resolver.resolve(&record)?;
    let models = ArchiveModels::new(&archive);
    let assembly = Assembly::build(&tree, &models, &AssemblyOptions::default())?;
    let interior = EntityInterior::build(&archive, &assembly, &tree, &InteriorOptions::default())?;
    Ok(InteriorFit::new(&assembly, &tree, &interior))
}

#[test]
fn every_container_is_measured_with_a_room_box_at_its_placement() -> TestResult {
    let fit = measured()?;

    assert_eq!(fit.containers().len(), 6);
    let front = &fit.containers()[0];
    let room = front.room().ok_or("the front container has no room box")?;
    assert!(room.contains([1.0, 6.0, 0.0]), "{room:?}");
    assert!(!room.contains([1.0, -20.0, 0.0]), "{room:?}");
    Ok(())
}

#[test]
fn a_ship_without_functional_hardpoints_has_none_to_place() -> TestResult {
    let fit = measured()?;

    assert_eq!(fit.hardpoints().len(), 0);
    assert_eq!(fit.pilot_seats().len(), 0);
    assert!(fit.containers().iter().all(|c| c.inside().is_empty()));
    Ok(())
}
