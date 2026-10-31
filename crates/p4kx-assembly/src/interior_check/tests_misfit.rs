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
                owner: 0,
                models: rooms.clone(),
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
fn a_displaced_cockpit_room_leaves_the_controller_outside_every_room() {
    let misfits = ship(50.0).misfits(TOLERANCE);

    assert_eq!(misfits, [Misfit::ControllerOutsideRooms { controller: 0 }]);
}

/// The hull of the synthetic ships: x from -3 to 3, y from -20 to 14, z from -1 to 5.
fn hull() -> Option<WorldBox> {
    WorldBox::around([[-3.0, -20.0, -1.0], [3.0, 14.0, 5.0]])
}

fn outside_hull(fit: &InteriorFit) -> Vec<usize> {
    fit.misfits(TOLERANCE)
        .into_iter()
        .filter_map(|misfit| match misfit {
            Misfit::RoomOutsideHull { container } => Some(container),
            _ => None,
        })
        .collect()
}

#[test]
fn a_room_moved_out_of_the_hull_is_a_misfit_though_it_holds_its_hardpoints() {
    let hardpoints = || {
        vec![
            hardpoint(HardpointKind::Controller, [0.0, 14.8, 1.0]),
            hardpoint(HardpointKind::Seat, [0.0, 15.3, 1.0]),
        ]
    };

    let beyond = fit(&[room_at(0.0), room_at(15.0)], hardpoints()).with_hull(hull());
    let within_slack = fit(&[room_at(0.0), room_at(12.5)], hardpoints()).with_hull(hull());
    let no_hull = fit(&[room_at(0.0), room_at(15.0)], hardpoints());

    assert_eq!(outside_hull(&beyond), [1]);
    assert_eq!(outside_hull(&within_slack), Vec::<usize>::new());
    assert_eq!(outside_hull(&no_hull), Vec::<usize>::new());
}

#[test]
fn a_room_without_room_models_cannot_protrude() {
    let empty = fit(&[room_at(0.0), None], Vec::new()).with_hull(hull());

    assert_eq!(outside_hull(&empty), Vec::<usize>::new());
}

#[test]
fn a_room_of_an_attached_entity_may_lie_outside_the_hull() {
    let rooms = [room_at(0.0), room_at(30.0)];
    let mut docked = fit(&rooms, Vec::new()).with_hull(hull());
    let own = fit(&rooms, Vec::new()).with_hull(hull());
    docked.containers[1].owner = 3;

    assert_eq!(outside_hull(&docked), Vec::<usize>::new());
    assert_eq!(outside_hull(&own), [1]);
}

#[test]
fn a_seat_away_from_the_controller_is_no_misfit() {
    let apart = fit(
        &[room_at(10.0), room_at(14.0)],
        vec![
            hardpoint(HardpointKind::Controller, [0.0, 10.0, 1.0]),
            hardpoint(HardpointKind::Seat, [0.0, 40.0, 1.0]),
        ],
    );

    assert_eq!(apart.misfits(TOLERANCE), []);
}

#[test]
fn a_cargo_grid_in_the_cockpit_room_is_no_misfit() {
    let shared = fit(
        &[room_at(10.0)],
        vec![
            hardpoint(HardpointKind::Controller, [0.0, 11.0, 1.0]),
            hardpoint(HardpointKind::CargoGrid, [0.0, 9.0, 1.0]),
        ],
    );

    assert_eq!(shared.misfits(TOLERANCE), []);
}

#[test]
fn a_cargo_grid_outside_every_room_is_a_misfit_when_another_grid_is_inside() {
    let lost = fit(
        &[room_at(10.0)],
        vec![
            hardpoint(HardpointKind::CargoGrid, [0.0, 10.0, 1.0]),
            hardpoint(HardpointKind::CargoGrid, [0.0, -10.0, 1.0]),
        ],
    );

    assert_eq!(
        lost.misfits(TOLERANCE),
        [Misfit::CargoGridOutsideRooms { grid: 1 }]
    );
}

#[test]
fn a_ship_whose_cargo_grids_all_lie_outside_every_room_is_not_checked() {
    let open = fit(
        &[room_at(10.0)],
        vec![
            hardpoint(HardpointKind::CargoGrid, [0.0, -10.0, 1.0]),
            hardpoint(HardpointKind::CargoGrid, [0.0, -12.0, 1.0]),
        ],
    );

    assert_eq!(open.misfits(TOLERANCE), []);
}

#[test]
fn the_tolerance_decides_how_far_outside_a_box_still_counts() {
    let near = fit(
        &[room_at(10.0)],
        vec![hardpoint(HardpointKind::Controller, [0.0, 12.3, 1.0])],
    );

    assert_eq!(near.misfits(TOLERANCE), []);
    assert_eq!(near.misfits(0.1).len(), 1);
}

#[test]
fn a_docked_ships_controller_is_not_checked() {
    let mut controller = hardpoint(HardpointKind::Controller, [0.0, 99.0, 0.0]);
    controller.vehicle = "collar/attach".to_owned();
    let docked = fit(&[room_at(10.0)], vec![controller]);

    assert_eq!(docked.misfits(TOLERANCE), []);
}

#[test]
fn a_container_without_a_room_box_holds_nothing() {
    let empty = fit(
        &[None],
        vec![hardpoint(HardpointKind::Controller, [0.0; 3])],
    );

    assert_eq!(empty.misfits(f32::MAX).len(), 1);
}

#[test]
fn non_finite_hardpoints_are_in_no_room() {
    let lost = fit(
        &[room_at(10.0)],
        vec![
            hardpoint(HardpointKind::Controller, [f32::NAN, 10.0, 1.0]),
            hardpoint(HardpointKind::CargoGrid, [0.0, 10.0, 1.0]),
            hardpoint(HardpointKind::CargoGrid, [f32::INFINITY, 0.0, 0.0]),
        ],
    );

    assert_eq!(
        lost.misfits(TOLERANCE),
        [
            Misfit::ControllerOutsideRooms { controller: 0 },
            Misfit::CargoGridOutsideRooms { grid: 2 }
        ]
    );
}
