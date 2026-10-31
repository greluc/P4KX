// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The census of the rules of `InteriorFit::misfits` over real ships, and the displacement of
//! single containers that shows which misplacements they catch.

use std::error::Error;

use p4kx_assembly::{ContainerFit, HardpointFit, HardpointKind, InteriorFit, Misfit, WorldBox};

use super::real_interior_hardpoints::{TOLERANCE, describe_misfits, measure, measure_displaced};

type TestResult = Result<(), Box<dyn Error>>;

/// Ships with interiors whose containers the census measures: the 15 found on LIVE. The class
/// name of the 600i Explorer is `ORIG_600i`.
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
    "ORIG_600i",
];

/// The census ships the Archive lacks or whose interior cannot be built.
const CENSUS_SKIPPED: [&str; 0] = [];

/// How far in metres a hardpoint may lie from a model box in the census. The farthest root
/// controller of the census, the Starfarer's, lies 0.87 m from its bridge container, so the margin
/// is 0.13 m and the 0.5 m of the Constellation and Cutlass tests does not hold for every ship.
const CENSUS_TOLERANCE: f32 = 1.0;

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

/// The distance of the farthest hardpoint of `kind` of the root vehicle to a model box.
fn farthest_root(fit: &InteriorFit, kind: HardpointKind) -> Option<f32> {
    fit.of_kind(kind)
        .filter(|at| at.hardpoint().vehicle().is_empty())
        .map(|at| at.distance().unwrap_or(f32::INFINITY))
        .max_by(f32::total_cmp)
}

/// The largest distance a root container protrudes beyond the hull model's box.
fn largest_overshoot(fit: &InteriorFit) -> Option<f32> {
    fit.hull()
        .into_iter()
        .flat_map(|hull| {
            fit.containers()
                .iter()
                .filter(|container| container.owner() == 0)
                .filter_map(|container| container.overshoot(hull))
        })
        .max_by(f32::total_cmp)
}

/// No rule of [`InteriorFit::misfits`] fires on any census ship at [`CENSUS_TOLERANCE`], and the
/// ships that could not be measured are exactly [`CENSUS_SKIPPED`]; prints per ship the smallest
/// room ratio, the largest dropped ratio, the largest root overshoot, the farthest root controller
/// and cargo grid, and the root cargo grids inside a model.
#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the census is printed for the test log"
)]
fn the_census_finds_no_misfit_on_any_real_ship() {
    let mut skipped = Vec::new();
    for class_name in CENSUS_SHIPS {
        let fit = match measure(class_name) {
            Ok(fit) => fit,
            Err(error) => {
                println!("census {class_name}: skipped: {error}");
                skipped.push(class_name);
                continue;
            }
        };
        let ratios: Vec<_> = fit.containers().iter().map(room_ratios).collect();
        let smallest_room = ratios.iter().filter_map(|r| r.0).min_by(f32::total_cmp);
        let largest_dropped = ratios.iter().filter_map(|r| r.1).max_by(f32::total_cmp);
        println!(
            "census {class_name}: {} containers, smallest room ratio {smallest_room:.3?}, largest dropped {largest_dropped:.3?}, largest root overshoot {:.2?} m, farthest root controller {:.2?} m, farthest root cargo grid {:.2?} m, {} root grids inside a model",
            fit.containers().len(),
            largest_overshoot(&fit),
            farthest_root(&fit, HardpointKind::Controller),
            farthest_root(&fit, HardpointKind::CargoGrid),
            fit.of_kind(HardpointKind::CargoGrid)
                .filter(|at| at.hardpoint().vehicle().is_empty() && at.is_inside())
                .count()
        );
        let described = describe_misfits(&fit, CENSUS_TOLERANCE).unwrap_or_else(|e| e.to_string());
        assert!(described.is_empty(), "{class_name}:\n{described}");
    }
    assert_eq!(skipped, CENSUS_SKIPPED, "ships that could not be measured");
}

/// Metres a container is moved along the hull axis in the displacement tests.
const DISPLACEMENT: f32 = 3.0;

/// The container holding the root controller, the one holding the first root cargo grid.
fn controller_and_cargo_containers(fit: &InteriorFit) -> Result<(usize, usize), &'static str> {
    let container_of = |kind: HardpointKind| {
        fit.of_kind(kind)
            .find(|at| at.hardpoint().vehicle().is_empty())
            .and_then(HardpointFit::container)
    };
    Ok((
        container_of(HardpointKind::Controller).ok_or("no container holds the root controller")?,
        container_of(HardpointKind::CargoGrid).ok_or("no container holds a root cargo grid")?,
    ))
}

/// The misfits of `class_name` with `container` moved by `distance` metres along the hull axis.
fn moved_misfits(
    class_name: &str,
    container: usize,
    distance: f32,
) -> Result<Vec<Misfit>, Box<dyn Error>> {
    Ok(measure_displaced(class_name, Some((container, [0.0, distance, 0.0])))?.misfits(TOLERANCE))
}

/// The Cutlass cockpit container also places the body model, so after a move of 3 m forwards the
/// box of all its models still holds the controller. Its room models leave the box of the hull
/// model, which is what the hull rule reports. A move backwards leaves the controller in the 8 m
/// long cockpit model and is not detected (ADR-0086).
#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_cutlass_cockpit_container_moved_forwards_along_the_hull_is_caught() -> TestResult {
    let class_name = "DRAK_Cutlass_Black";
    let (cockpit, _) = controller_and_cargo_containers(&measure(class_name)?)?;
    assert_eq!(
        moved_misfits(class_name, cockpit, DISPLACEMENT)?,
        [Misfit::RoomOutsideHull { container: cockpit }],
        "{class_name}: the cockpit container moved by {DISPLACEMENT} m"
    );
    Ok(())
}

/// The Constellation's cockpit container moved 3 m forwards leaves the hull and the hull rule
/// reports it. Moved 3 m backwards, or its cargo container moved either way, nothing is reported:
/// the controller and the cargo grid still lie inside a model, since the 9 m front and 22 m body
/// models are much longer than the move, and no room leaves the hull. ADR-0086 records these
/// three as not detected.
#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_constellation_container_moved_along_the_hull_is_caught_only_where_it_leaves_the_hull()
-> TestResult {
    let class_name = "RSI_Constellation_Andromeda";
    let (cockpit, cargo) = controller_and_cargo_containers(&measure(class_name)?)?;
    let outcome = |container, distance| moved_misfits(class_name, container, distance);

    assert_eq!(
        outcome(cockpit, DISPLACEMENT)?,
        [Misfit::RoomOutsideHull { container: cockpit }]
    );
    assert_eq!(outcome(cockpit, -DISPLACEMENT)?, []);
    assert_eq!(outcome(cargo, DISPLACEMENT)?, []);
    assert_eq!(outcome(cargo, -DISPLACEMENT)?, []);
    Ok(())
}
