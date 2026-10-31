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
            let rooms: Vec<WorldBox> = room.iter().copied().collect();
            let (inside, outside) = classify(&rooms, &hardpoints);
            ContainerFit {
                file: "room.socpak".to_owned(),
                room: *room,
                rooms,
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
        &[room_at(cockpit_y), room_at(6.0)],
        vec![
            hardpoint(HardpointKind::Controller, [0.0, 11.0, 1.0]),
            hardpoint(HardpointKind::Seat, [0.0, 10.0, 1.0]),
            hardpoint(HardpointKind::CargoGrid, [0.0, 6.0, 1.0]),
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
        [
            Misfit::CockpitSplit {
                controller: 0,
                seat: 1
            },
            Misfit::SeatOutsideRooms { seat: 1 },
            Misfit::RoomsApart { container: 0 },
            Misfit::RoomsApart { container: 1 }
        ]
    );
}

#[test]
fn a_room_moved_along_the_hull_away_from_its_neighbours_is_apart_though_it_holds_its_hardpoints() {
    let gap = fit(
        &[room_at(10.0), room_at(14.9)],
        vec![
            hardpoint(HardpointKind::Controller, [0.0, 10.0, 1.0]),
            hardpoint(HardpointKind::Seat, [0.0, 10.5, 1.0]),
        ],
    );

    assert_eq!(
        gap.misfits(TOLERANCE),
        [
            Misfit::RoomsApart { container: 0 },
            Misfit::RoomsApart { container: 1 }
        ]
    );
    assert_eq!(gap.misfits(1.0), []);
}

#[test]
fn a_chain_of_rooms_is_not_apart_and_a_lone_room_is_not_either() {
    let chain = fit(&[room_at(0.0), room_at(4.0), room_at(8.0)], Vec::new());
    let lone = fit(&[room_at(0.0), None], Vec::new());

    assert_eq!(chain.misfits(TOLERANCE), []);
    assert_eq!(lone.misfits(TOLERANCE), []);
}

#[test]
fn a_seat_beyond_the_room_it_belongs_to_is_a_misfit_though_another_seat_is_nearer_the_controller() {
    let moved = fit(
        &[room_at(10.0)],
        vec![
            hardpoint(HardpointKind::Controller, [0.0, 10.0, 1.0]),
            hardpoint(HardpointKind::Seat, [0.0, 9.0, 1.0]),
            hardpoint(HardpointKind::Seat, [0.0, 14.0, 1.0]),
        ],
    );

    assert_eq!(
        moved.misfits(TOLERANCE),
        [Misfit::SeatOutsideRooms { seat: 2 }]
    );
}

#[test]
fn a_controller_and_seat_in_different_rooms_are_a_split() {
    let split = fit(
        &[room_at(10.0), room_at(14.0)],
        vec![
            hardpoint(HardpointKind::Controller, [0.0, 10.0, 1.0]),
            hardpoint(HardpointKind::Seat, [0.0, 14.0, 1.0]),
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
    assert_eq!(near.misfits(0.1).len(), 2);
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

    assert_eq!(empty.misfits(f32::MAX).len(), 2);
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
