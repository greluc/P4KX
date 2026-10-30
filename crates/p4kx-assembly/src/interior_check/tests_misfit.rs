// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Tests of the rules a correct placement keeps, on boxes and hardpoints made by hand.

use super::fit::{ContainerFit, classify};
use super::tests::hardpoint;
use super::*;

const TOLERANCE: f32 = 0.5;

/// A room of 4 m by 4 m by 4 m standing on `y`, floor at z = 0.
fn room_at(y: f32) -> Option<WorldBox> {
    WorldBox::around([[-2.0, y - 2.0, 0.0], [2.0, y + 2.0, 4.0]])
}

fn fit(rooms: &[Option<WorldBox>], hardpoints: Vec<Hardpoint>) -> InteriorFit {
    let containers = rooms
        .iter()
        .map(|room| {
            let (inside, outside) = classify(room.as_ref(), &hardpoints);
            ContainerFit {
                file: "room.socpak".to_owned(),
                room: *room,
                inside,
                outside,
                animated: Vec::new(),
                unreadable: 0,
            }
        })
        .collect();
    InteriorFit::from_measured(containers, hardpoints, Vec::new())
}

fn ship(cockpit_y: f32) -> InteriorFit {
    fit(
        &[room_at(cockpit_y), room_at(-10.0)],
        vec![
            hardpoint(HardpointKind::Controller, [0.0, 11.0, 1.0]),
            hardpoint(HardpointKind::Seat, [0.0, 10.0, 1.0]),
            hardpoint(HardpointKind::CargoGrid, [0.0, -10.0, 1.0]),
        ],
    )
}

#[test]
fn rooms_over_their_hardpoints_break_no_rule() {
    assert_eq!(ship(10.0).misfits(TOLERANCE), []);
}

#[test]
fn a_displaced_cockpit_room_splits_the_controller_and_seat_from_every_room() {
    let misfits = ship(50.0).misfits(TOLERANCE);

    assert_eq!(
        misfits,
        [Misfit::CockpitSplit {
            controller: 0,
            seat: 1
        }]
    );
}

#[test]
fn a_controller_and_seat_in_different_rooms_are_a_split() {
    let split = fit(
        &[room_at(10.0), room_at(20.0)],
        vec![
            hardpoint(HardpointKind::Controller, [0.0, 10.0, 1.0]),
            hardpoint(HardpointKind::Seat, [0.0, 20.0, 1.0]),
        ],
    );

    assert_eq!(
        split.misfits(TOLERANCE),
        [Misfit::CockpitSplit {
            controller: 0,
            seat: 1
        }]
    );
}

#[test]
fn a_cargo_grid_in_the_cockpit_room_is_not_in_a_cargo_room_of_its_own() {
    let merged = fit(
        &[room_at(10.0), room_at(10.0)],
        vec![
            hardpoint(HardpointKind::Controller, [0.0, 11.0, 1.0]),
            hardpoint(HardpointKind::Seat, [0.0, 10.0, 1.0]),
            hardpoint(HardpointKind::CargoGrid, [0.0, 9.0, 1.0]),
        ],
    );

    assert_eq!(
        merged.misfits(TOLERANCE),
        [Misfit::CargoOutsideCargoRoom { grid: 2 }]
    );
}

#[test]
fn a_cargo_grid_outside_every_room_is_a_misfit() {
    let lost = fit(
        &[room_at(10.0)],
        vec![hardpoint(HardpointKind::CargoGrid, [0.0, -10.0, 1.0])],
    );

    assert_eq!(
        lost.misfits(TOLERANCE),
        [Misfit::CargoOutsideCargoRoom { grid: 0 }]
    );
}

#[test]
fn the_tolerance_decides_how_far_outside_a_box_still_counts() {
    let near = fit(
        &[room_at(10.0)],
        vec![
            hardpoint(HardpointKind::Controller, [0.0, 10.0, 1.0]),
            hardpoint(HardpointKind::Seat, [0.0, 12.3, 1.0]),
        ],
    );

    assert_eq!(near.misfits(TOLERANCE), []);
    assert_eq!(near.misfits(0.1).len(), 1);
}

#[test]
fn a_docked_ships_controller_is_not_checked() {
    let mut controller = hardpoint(HardpointKind::Controller, [0.0, 99.0, 0.0]);
    controller.vehicle = "collar/attach".to_owned();
    let docked = fit(
        &[room_at(10.0)],
        vec![controller, {
            let mut seat = hardpoint(HardpointKind::Seat, [0.0, 10.0, 1.0]);
            seat.vehicle = "collar/attach".to_owned();
            seat
        }],
    );

    assert_eq!(docked.misfits(TOLERANCE), []);
}

#[test]
fn a_container_without_a_room_box_holds_nothing() {
    let empty = fit(
        &[None],
        vec![
            hardpoint(HardpointKind::Controller, [0.0; 3]),
            hardpoint(HardpointKind::Seat, [0.0; 3]),
        ],
    );

    assert_eq!(empty.misfits(f32::MAX).len(), 1);
}

#[test]
fn non_finite_hardpoints_are_in_no_room() {
    let lost = fit(
        &[room_at(10.0)],
        vec![
            hardpoint(HardpointKind::Controller, [f32::NAN, 10.0, 1.0]),
            hardpoint(HardpointKind::Seat, [0.0, 10.0, 1.0]),
            hardpoint(HardpointKind::CargoGrid, [f32::INFINITY, 0.0, 0.0]),
        ],
    );

    assert_eq!(
        lost.misfits(TOLERANCE),
        [Misfit::CargoOutsideCargoRoom { grid: 2 }]
    );
}
