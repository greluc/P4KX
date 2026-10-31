// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The containers of a real ship's Interior hold the hardpoints they serve: the root vehicle's
//! flight controller and pilot seat lie in one container, every cargo grid in a container without
//! cockpit hardpoints, and doors and animated parts at the rooms they close.

use std::error::Error;
use std::fmt::Write as _;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, EntityInterior, HardpointFit,
    HardpointKind, InteriorFit, InteriorOptions, LoadoutResolver, Misfit,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

/// How far in metres a hardpoint's origin may lie outside a room box and still count as in it:
/// an Entity's origin can sit on a room's floor or wall, which the box of its models only touches.
const TOLERANCE: f32 = 0.5;

/// Measures `class_name` with the container `displaced.0` moved by `displaced.1` metres.
fn measure_displaced(
    class_name: &str,
    displaced: Option<(usize, [f32; 3])>,
) -> Result<InteriorFit, Box<dyn Error>> {
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
    Ok(InteriorFit::new_displaced(
        &assembly, &tree, &interior, displaced,
    ))
}

fn measure(class_name: &str) -> Result<InteriorFit, Box<dyn Error>> {
    measure_displaced(class_name, None)
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

fn describe_index(fit: &InteriorFit, index: usize) -> String {
    fit.hardpoints()
        .get(index)
        .map_or_else(|| format!("hardpoint {index}"), |at| describe(fit, at))
}

#[expect(
    clippy::print_stdout,
    reason = "the report is printed for the test log"
)]
fn report(class_name: &str, fit: &InteriorFit) {
    println!("== {class_name}");
    for container in fit.containers() {
        println!(
            "container {} room {:?}, {} unreadable models, holds {:?}",
            container.file(),
            container.room(),
            container.unreadable_models(),
            container.inside()
        );
        for room in container.rooms() {
            println!("  room model {room:?}");
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

/// Checks the containers of `class_name`: the rules of [`InteriorFit::misfits`], which hold for
/// the root vehicle (a docked ship's flight controller sits at that ship's origin, below its
/// cockpit), and doors and animated parts within [`TOLERANCE`] of a room.
fn check(class_name: &str) -> TestResult {
    let fit = measure(class_name)?;
    report(class_name, &fit);
    assert!(
        fit.unplaced().is_empty(),
        "{class_name}: {:?}",
        fit.unplaced()
    );
    let roots = fit
        .of_kind(HardpointKind::Controller)
        .filter(|at| at.hardpoint().vehicle().is_empty())
        .count();
    assert!(roots > 0, "{class_name}: no controller of the root vehicle");
    assert!(!fit.pilot_seats().is_empty(), "{class_name}: no pilot seat");
    assert!(
        fit.of_kind(HardpointKind::CargoGrid).count() > 0,
        "{class_name}: no cargo grid"
    );
    let failures = failures(&fit)?;
    assert!(
        failures.is_empty(),
        "{class_name}: displaced beyond {TOLERANCE} m:\n{failures}"
    );
    Ok(())
}

/// Describes every rule of [`InteriorFit::misfits`] that is broken, every door and every
/// animated part that lies more than [`TOLERANCE`] from a room; empty when all hold.
fn failures(fit: &InteriorFit) -> Result<String, Box<dyn Error>> {
    let mut failures = String::new();
    for misfit in fit.misfits(TOLERANCE) {
        match misfit {
            Misfit::CockpitSplit { controller, seat } => writeln!(
                failures,
                "no container holds both\n  {}\n  {}",
                describe_index(fit, controller),
                describe_index(fit, seat)
            )?,
            Misfit::CargoOutsideCargoRoom { grid } => writeln!(
                failures,
                "no cargo room holds\n  {}",
                describe_index(fit, grid)
            )?,
            Misfit::SeatOutsideRooms { seat } => {
                writeln!(failures, "no room holds\n  {}", describe_index(fit, seat))?;
            }
            _ => writeln!(failures, "unknown misfit {misfit:?}")?,
        }
    }
    for at in fit
        .of_kind(HardpointKind::Door)
        .filter(|at| !at.is_within(TOLERANCE))
    {
        writeln!(failures, "{}", describe(fit, at))?;
    }
    for container in fit.containers() {
        for part in container.animated() {
            if part.distance().is_none_or(|distance| distance > TOLERANCE) {
                writeln!(
                    failures,
                    "animated {} of {} at {:?}: {:?} m from the room",
                    part.name(),
                    container.file(),
                    part.position(),
                    part.distance()
                )?;
            }
        }
    }
    Ok(failures)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_cutlass_containers_hold_their_controller_seat_and_cargo_grids() -> TestResult {
    check("DRAK_Cutlass_Black")
}

/// Metres the cockpit container is moved along the hull axis in the displacement test.
const DISPLACEMENT: f32 = 3.0;

/// The Cutlass cockpit container also places the body model, so the box of all its models still
/// holds the controller and the seats after a move of 3 m forwards. The room models do not: the
/// pilot seat leaves the cockpit model when it moves backwards, and the container's rooms leave
/// those of the body container when it moves forwards.
#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_cutlass_cockpit_container_moved_along_the_hull_is_caught() -> TestResult {
    let class_name = "DRAK_Cutlass_Black";
    let fit = measure(class_name)?;
    let cockpit = fit
        .of_kind(HardpointKind::Controller)
        .find(|at| at.hardpoint().vehicle().is_empty())
        .and_then(HardpointFit::container)
        .ok_or("no container holds the root controller")?;
    for shift in [DISPLACEMENT, -DISPLACEMENT] {
        let moved = measure_displaced(class_name, Some((cockpit, [0.0, shift, 0.0])))?;
        assert!(
            !failures(&moved)?.is_empty(),
            "{class_name}: the cockpit container moved by {shift} m still fits"
        );
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_constellation_containers_hold_their_controller_seat_and_cargo_grids() -> TestResult {
    check("RSI_Constellation_Andromeda")
}
