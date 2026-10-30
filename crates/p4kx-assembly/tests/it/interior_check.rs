// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The check of an Interior's containers against the hardpoints of its Assembly, on synthetic
//! ships whose rooms are 4 m cubes.

use p4kx_assembly::{
    ArchiveModels, Assembly, AssemblyOptions, EntityInterior, HardpointKind, InteriorFit,
    InteriorOptions, LoadoutResolver, Misfit,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{
    ENTITY_FIT_CLASS, ENTITY_FIT_MERGED_CLASS, ENTITY_FIT_MOVED_CLASS, entity_fit_entries,
};

use crate::assembly_support::{TestResult, fixture};

const TOLERANCE: f32 = 0.5;

fn measured(class_name: &str) -> Result<InteriorFit, Box<dyn std::error::Error>> {
    let fixture = fixture(entity_fit_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find(class_name)?;
    let tree = resolver.resolve(&record)?;
    let models = ArchiveModels::new(&archive);
    let assembly = Assembly::build(&tree, &models, &AssemblyOptions::default())?;
    let interior = EntityInterior::build(&archive, &assembly, &tree, &InteriorOptions::default())?;
    Ok(InteriorFit::new(&assembly, &tree, &interior))
}

fn holders(fit: &InteriorFit, kind: HardpointKind) -> Result<Vec<usize>, String> {
    let at = fit
        .of_kind(kind)
        .next()
        .ok_or_else(|| format!("no {} hardpoint", kind.code()))?;
    Ok(fit
        .containers()
        .iter()
        .enumerate()
        .filter(|(_, container)| container.holds(at.hardpoint().position(), 0.0))
        .map(|(index, _)| index)
        .collect())
}

#[test]
fn rooms_over_their_hardpoints_hold_them_each_in_its_own_room() -> TestResult {
    let fit = measured(ENTITY_FIT_CLASS)?;

    assert_eq!(fit.containers().len(), 2);
    assert_eq!(fit.hardpoints().len(), 3);
    assert_eq!(fit.pilot_seats().len(), 1);
    assert_eq!(holders(&fit, HardpointKind::Seat)?, [0]);
    assert_eq!(holders(&fit, HardpointKind::Controller)?, [0]);
    assert_eq!(holders(&fit, HardpointKind::CargoGrid)?, [1]);
    assert_eq!(fit.misfits(TOLERANCE), []);
    Ok(())
}

#[test]
fn a_cockpit_room_moved_away_leaves_the_seat_and_controller_outside_every_room() -> TestResult {
    let fit = measured(ENTITY_FIT_MOVED_CLASS)?;

    assert_eq!(holders(&fit, HardpointKind::Seat)?, Vec::<usize>::new());
    assert_eq!(holders(&fit, HardpointKind::CargoGrid)?, [1]);
    assert!(matches!(
        fit.misfits(TOLERANCE).as_slice(),
        [Misfit::CockpitSplit { .. }]
    ));
    Ok(())
}

#[test]
fn rooms_put_on_the_cockpit_leave_no_cargo_room_for_a_grid_in_it() -> TestResult {
    let fit = measured(ENTITY_FIT_MERGED_CLASS)?;

    assert_eq!(holders(&fit, HardpointKind::CargoGrid)?, [0, 1]);
    assert!(matches!(
        fit.misfits(TOLERANCE).as_slice(),
        [Misfit::CargoOutsideCargoRoom { .. }]
    ));
    Ok(())
}
