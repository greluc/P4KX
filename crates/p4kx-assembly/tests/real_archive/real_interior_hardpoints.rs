// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The containers of a real ship's Interior hold the hardpoints they serve: the root vehicle's
//! flight controller and the cargo grids lie in containers, no room of the root Entity leaves
//! the hull, and doors and animated parts lie at the rooms they close.

use std::error::Error;
use std::fmt::Write as _;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, ContainerFit, EntityInterior,
    HardpointFit, HardpointKind, InteriorFit, InteriorOptions, LoadoutResolver, Misfit, WorldBox,
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
        for model in container
            .models()
            .iter()
            .filter(|model| !container.rooms().contains(model))
        {
            println!("  dropped model {model:?}, volume {} m3", model.volume());
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
            Misfit::ControllerOutsideRooms { controller } => writeln!(
                failures,
                "no container holds\n  {}",
                describe_index(fit, controller)
            )?,
            Misfit::CargoGridOutsideRooms { grid } => writeln!(
                failures,
                "no container holds\n  {}",
                describe_index(fit, grid)
            )?,
            Misfit::RoomOutsideHull { container } => writeln!(
                failures,
                "room models outside the hull: {}",
                fit.containers()
                    .get(container)
                    .map_or("unknown container", |container| container.file())
            )?,
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

/// The Cutlass cockpit container also places the body model, so after a move of 3 m forwards the
/// box of all its models still holds the controller and the seats. Its room models leave the box
/// of the hull model, which is what the hull rule reports. A move backwards leaves the controller
/// and the nearest seat in the 8 m long cockpit model and is not detected.
#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_cutlass_cockpit_container_moved_forwards_along_the_hull_is_caught() -> TestResult {
    let class_name = "DRAK_Cutlass_Black";
    let fit = measure(class_name)?;
    let cockpit = fit
        .of_kind(HardpointKind::Controller)
        .find(|at| at.hardpoint().vehicle().is_empty())
        .and_then(HardpointFit::container)
        .ok_or("no container holds the root controller")?;
    let moved = measure_displaced(class_name, Some((cockpit, [0.0, DISPLACEMENT, 0.0])))?;
    assert_eq!(
        moved.misfits(TOLERANCE),
        [Misfit::RoomOutsideHull { container: cockpit }],
        "{class_name}: the cockpit container moved by {DISPLACEMENT} m"
    );
    Ok(())
}

/// Ships with interiors whose containers the census measures; a class the Archive lacks or
/// whose interior cannot be built is skipped.
const CENSUS_SHIPS: [&str; 16] = [
    "DRAK_Caterpillar",
    "ORIG_890Jump",
    "MISC_Hull_C",
    "MISC_Starfarer",
    "ANVL_Carrack",
    "RSI_Constellation_Andromeda",
    "RSI_Constellation_Aquila",
    "DRAK_Cutlass_Black",
    "RSI_Polaris",
    "AEGS_Idris_P",
    "ANVL_Valkyrie",
    "DRAK_Corsair",
    "MISC_Freelancer",
    "CRUS_Starlifter_C2",
    "AEGS_Redeemer",
    "ORIG_600i_Explorer",
];

/// The ratios of a container's room models and dropped models to its largest static model, as
/// the smallest room and the largest dropped.
fn room_ratios(container: &ContainerFit) -> (Option<f32>, Option<f32>) {
    let largest = container
        .models()
        .iter()
        .map(WorldBox::volume)
        .fold(0.0, f32::max);
    let ratio = |room: bool| {
        container
            .models()
            .iter()
            .filter(move |model| container.rooms().contains(model) == room)
            .map(move |model| model.volume() / largest)
    };
    (
        ratio(true).min_by(f32::total_cmp),
        ratio(false).max_by(f32::total_cmp),
    )
}

/// How far in metres a root controller may lie from a model box in the census: the Starfarer's
/// lies 0.87 m outside its bridge container, so the 0.5 m of the Constellation and Cutlass tests
/// does not hold for every ship.
const CENSUS_TOLERANCE: f32 = 1.0;

/// The distance of the farthest hardpoint of `kind` to a model box, over the hardpoints of the root
/// vehicle only when `root`.
fn farthest(fit: &InteriorFit, kind: HardpointKind, root: bool) -> Option<f32> {
    fit.of_kind(kind)
        .filter(|at| !root || at.hardpoint().vehicle().is_empty())
        .map(|at| at.distance().unwrap_or(f32::INFINITY))
        .max_by(f32::total_cmp)
}

/// No rule of [`InteriorFit::misfits`] fires on any census ship; prints per ship the smallest room
/// ratio, the largest dropped ratio, the largest root overshoot and the farthest controller and
/// cargo grid.
#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the census is printed for the test log"
)]
fn the_census_finds_no_misfit_on_any_real_ship() {
    let mut measured = 0;
    for class_name in CENSUS_SHIPS {
        let Ok(fit) = measure(class_name) else {
            println!("census {class_name}: skipped");
            continue;
        };
        measured += 1;
        let ratios: Vec<_> = fit.containers().iter().map(room_ratios).collect();
        let smallest_room = ratios.iter().filter_map(|r| r.0).min_by(f32::total_cmp);
        let largest_dropped = ratios.iter().filter_map(|r| r.1).max_by(f32::total_cmp);
        let overshoot = fit
            .hull()
            .into_iter()
            .flat_map(|hull| {
                fit.containers()
                    .iter()
                    .filter(|container| container.owner() == 0)
                    .filter_map(|container| container.overshoot(hull))
            })
            .max_by(f32::total_cmp);
        println!(
            "census {class_name}: {} containers, smallest room ratio {smallest_room:.3?}, largest dropped {largest_dropped:.3?}, largest root overshoot {overshoot:.2?} m, farthest root controller {:.2?} m, farthest cargo grid {:.2?} m, {} grids inside a model",
            fit.containers().len(),
            farthest(&fit, HardpointKind::Controller, true),
            farthest(&fit, HardpointKind::CargoGrid, false),
            fit.of_kind(HardpointKind::CargoGrid)
                .filter(|at| at.is_within(0.0))
                .count()
        );
        let misfits = fit.misfits(CENSUS_TOLERANCE);
        assert!(misfits.is_empty(), "{class_name}: {misfits:?}");
    }
    assert!(
        measured >= 5,
        "only {measured} census ships could be measured"
    );
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_constellation_containers_hold_their_controller_seat_and_cargo_grids() -> TestResult {
    check("RSI_Constellation_Andromeda")
}
