// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The containers of a real ship's Interior hold the hardpoints they serve: the pilot seat and
//! flight controller lie in a cockpit's room box, the cargo grids in a cargo room's box.

use std::error::Error;
use std::fmt::Write as _;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, EntityInterior, HardpointFit,
    HardpointKind, InteriorFit, InteriorOptions, LoadoutResolver,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const TOLERANCE: f32 = 0.5;

fn measure(class_name: &str) -> Result<InteriorFit, Box<dyn Error>> {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let record = resolver.entities().find(class_name)?;
    let tree = resolver.resolve(&record)?;
    let models = ArchiveModels::new(&archive);
    let assembly = Assembly::build(&tree, &models, &AssemblyOptions::default())?;
    let interior = EntityInterior::build(&archive, &assembly, &tree, &InteriorOptions::default())?;
    Ok(InteriorFit::new(&assembly, &tree, &interior))
}

fn describe(fit: &InteriorFit, at: &HardpointFit) -> String {
    let hardpoint = at.hardpoint();
    let container = at
        .container()
        .and_then(|index| fit.containers().get(index))
        .map_or("no container", |container| container.file());
    format!(
        "{} {} ({}) at {:?}: {:?} m from {container}",
        hardpoint.kind().code(),
        hardpoint.class_name(),
        hardpoint.path(),
        hardpoint.position(),
        at.distance()
    )
}

#[expect(clippy::print_stdout, reason = "the numbers go into the PR body")]
fn report(class_name: &str, fit: &InteriorFit) {
    println!("== {class_name}");
    for container in fit.containers() {
        println!("container {} room {:?}", container.file(), container.room());
        for &index in container.inside() {
            if let Some(at) = fit.hardpoints().get(index) {
                println!("  inside  {}", describe(fit, at));
            }
        }
        for part in container.animated() {
            println!(
                "  animated {} {:?} at {:?}: {:?} m from the room",
                part.name(),
                part.model_path(),
                part.position(),
                part.distance()
            );
        }
    }
    for at in fit.hardpoints() {
        println!("hardpoint {}", describe(fit, at));
    }
}

fn check(class_name: &str) -> TestResult {
    let fit = measure(class_name)?;
    report(class_name, &fit);
    // A docked ship's flight controller sits at that ship's origin, below its cockpit.
    let controllers: Vec<&HardpointFit> = fit
        .of_kind(HardpointKind::Controller)
        .filter(|at| at.hardpoint().vehicle().is_empty())
        .collect();
    let pilot_seats = fit.pilot_seats();
    let grids: Vec<&HardpointFit> = fit.of_kind(HardpointKind::CargoGrid).collect();
    assert!(!controllers.is_empty(), "{class_name}: no controller");
    assert!(!pilot_seats.is_empty(), "{class_name}: no pilot seat");
    assert!(!grids.is_empty(), "{class_name}: no cargo grid");
    let doors: Vec<&HardpointFit> = fit.of_kind(HardpointKind::Door).collect();
    let served = controllers
        .iter()
        .chain(&pilot_seats)
        .chain(&grids)
        .chain(&doors);
    let mut misplaced = String::new();
    for at in served.filter(|at| !at.is_within(TOLERANCE)) {
        writeln!(misplaced, "{}", describe(&fit, at))?;
    }
    for container in fit.containers() {
        for part in container.animated() {
            if part.distance().is_none_or(|distance| distance > TOLERANCE) {
                writeln!(
                    misplaced,
                    "animated {} of {} at {:?}: {:?} m from the room",
                    part.name(),
                    container.file(),
                    part.position(),
                    part.distance()
                )?;
            }
        }
    }
    assert!(
        misplaced.is_empty(),
        "{class_name}: outside every room box by more than {TOLERANCE} m:\n{misplaced}"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_cutlass_containers_hold_their_controller_seat_and_cargo_grids() -> TestResult {
    check("DRAK_Cutlass_Black")
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_constellation_containers_hold_their_controller_seat_and_cargo_grids() -> TestResult {
    check("RSI_Constellation_Andromeda")
}
