// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::fit::{ContainerFit, HardpointFit};
use super::*;

const NAN: f32 = f32::NAN;

pub(super) fn unit_box() -> WorldBox {
    WorldBox::around([[0.0; 3], [1.0; 3]]).expect("two finite points")
}

pub(super) fn hardpoint(kind: HardpointKind, position: [f32; 3]) -> Hardpoint {
    Hardpoint {
        path: "hp".to_owned(),
        class_name: "Class".to_owned(),
        kind,
        position,
        vehicle: String::new(),
    }
}

pub(super) fn container(room: Option<WorldBox>) -> ContainerFit {
    ContainerFit {
        file: "a.socpak".to_owned(),
        room,
        inside: Vec::new(),
        outside: Vec::new(),
        animated: Vec::new(),
        unreadable: 0,
    }
}

#[test]
fn a_box_around_no_finite_point_does_not_exist() {
    assert_eq!(WorldBox::around([]), None);
    assert_eq!(
        WorldBox::around([[NAN, 0.0, 0.0], [0.0, f32::INFINITY, 0.0]]),
        None
    );
    assert_eq!(WorldBox::point([0.0, f32::NEG_INFINITY, 0.0]), None);
}

#[test]
fn non_finite_points_are_skipped_when_a_box_grows() {
    let bounds = WorldBox::around([[NAN, 5.0, 5.0], [1.0, 2.0, 3.0], [-1.0, f32::INFINITY, 0.0]]);

    let bounds = bounds.expect("one finite point");
    assert_eq!(bounds.min(), [1.0, 2.0, 3.0]);
    assert_eq!(bounds.max(), [1.0, 2.0, 3.0]);
}

#[test]
fn a_degenerate_box_contains_its_own_points_and_nothing_else() {
    let point = WorldBox::point([1.0, 2.0, 3.0]).expect("finite");
    let flat = WorldBox::around([[0.0, 0.0, 2.0], [4.0, 4.0, 2.0]]).expect("finite");

    assert!(point.contains([1.0, 2.0, 3.0]));
    assert!(!point.contains([1.0, 2.0, 3.1]));
    assert!(flat.contains([2.0, 3.0, 2.0]));
    assert_eq!(flat.distance([2.0, 3.0, 5.0]), Some(3.0));
}

#[test]
fn the_distance_to_a_box_is_zero_inside_and_euclidean_outside() {
    let bounds = unit_box();

    assert_eq!(bounds.distance([0.5; 3]), Some(0.0));
    assert_eq!(bounds.distance([1.0, 0.0, 0.5]), Some(0.0));
    assert_eq!(bounds.distance([4.0, 5.0, 0.5]), Some(5.0));
    assert_eq!(bounds.distance([-2.0, 0.5, 0.5]), Some(2.0));
}

#[test]
fn a_non_finite_point_is_in_no_box_and_has_no_distance() {
    let bounds = unit_box();

    for point in [
        [NAN; 3],
        [0.5, f32::INFINITY, 0.5],
        [f32::NEG_INFINITY, 0.0, 0.0],
    ] {
        assert!(!bounds.contains(point));
        assert_eq!(bounds.distance(point), None);
    }
}

#[test]
fn a_distance_that_overflows_is_unknown() {
    let bounds = unit_box();

    assert_eq!(bounds.distance([f32::MAX, f32::MAX, 0.0]), None);
}

#[test]
fn item_types_tell_the_kind_of_hardpoint() {
    assert_eq!(
        HardpointKind::of_item_type("FlightController"),
        Some(HardpointKind::Controller)
    );
    assert_eq!(
        HardpointKind::of_item_type("Seat"),
        Some(HardpointKind::Seat)
    );
    assert_eq!(
        HardpointKind::of_item_type("CargoGrid"),
        Some(HardpointKind::CargoGrid)
    );
    assert_eq!(
        HardpointKind::of_item_type("Door"),
        Some(HardpointKind::Door)
    );
    assert_eq!(
        HardpointKind::of_item_type("DockingAnimator"),
        Some(HardpointKind::Door)
    );
    assert_eq!(HardpointKind::of_item_type("WeaponGun"), None);
    assert_eq!(HardpointKind::of_item_type(""), None);
}

#[test]
fn a_hardpoint_is_measured_against_the_nearest_box() {
    let far = WorldBox::around([[10.0; 3], [11.0; 3]]);
    let containers = [container(far), container(None), container(Some(unit_box()))];

    let fit = HardpointFit::measure(hardpoint(HardpointKind::Seat, [0.5, 0.5, 2.0]), &containers);

    assert_eq!(fit.container(), Some(2));
    assert_eq!(fit.distance(), Some(1.0));
    assert!(fit.is_within(1.0));
    assert!(!fit.is_within(0.5));
}

#[test]
fn a_hardpoint_without_a_box_or_a_finite_position_is_within_nothing() {
    let none = HardpointFit::measure(hardpoint(HardpointKind::Seat, [0.0; 3]), &[container(None)]);
    let lost = HardpointFit::measure(
        hardpoint(HardpointKind::Seat, [NAN, 0.0, 0.0]),
        &[container(Some(unit_box()))],
    );

    for fit in [none, lost] {
        assert_eq!(fit.container(), None);
        assert_eq!(fit.distance(), None);
        assert!(!fit.is_within(f32::MAX));
    }
}

fn fit_of(hardpoints: Vec<Hardpoint>) -> InteriorFit {
    InteriorFit {
        containers: Vec::new(),
        unplaced: Vec::new(),
        hardpoints: hardpoints
            .into_iter()
            .map(|hardpoint| HardpointFit::measure(hardpoint, &[]))
            .collect(),
    }
}

#[test]
fn the_pilot_seat_is_the_seat_nearest_to_a_controller_once() {
    let fit = fit_of(vec![
        hardpoint(HardpointKind::Seat, [0.0, 0.0, 0.0]),
        hardpoint(HardpointKind::Controller, [0.0, 4.5, 0.0]),
        hardpoint(HardpointKind::Seat, [0.0, 5.0, 0.0]),
        hardpoint(HardpointKind::Seat, [NAN, 4.5, 0.0]),
        hardpoint(HardpointKind::Controller, [0.0, 4.0, 0.0]),
    ]);

    let seats = fit.pilot_seats();

    assert_eq!(seats.len(), 1);
    assert_eq!(seats[0].hardpoint().position(), [0.0, 5.0, 0.0]);
}

#[test]
fn a_pilot_seat_belongs_to_the_vehicle_of_its_controller() {
    let mut docked = hardpoint(HardpointKind::Seat, [0.0, 1.0, 0.0]);
    docked.vehicle = "collar/attach".to_owned();
    let fit = fit_of(vec![
        hardpoint(HardpointKind::Controller, [0.0; 3]),
        docked,
        hardpoint(HardpointKind::Seat, [0.0, 9.0, 0.0]),
    ]);

    let seats = fit.pilot_seats();

    assert_eq!(seats.len(), 1);
    assert_eq!(seats[0].hardpoint().position(), [0.0, 9.0, 0.0]);
}

#[test]
fn a_controller_without_a_seat_has_no_pilot_seat() {
    let fit = fit_of(vec![hardpoint(HardpointKind::Controller, [0.0; 3])]);

    assert_eq!(fit.pilot_seats().len(), 0);
    assert_eq!(fit.of_kind(HardpointKind::Controller).count(), 1);
}

#[test]
fn only_cga_models_are_animated() {
    use super::fit::is_animated;

    assert!(is_animated(Some("objects/ships/door.cga")));
    assert!(is_animated(Some("Objects\\Ships\\DOOR.CGA")));
    assert!(!is_animated(Some("objects/ships/room.cgf")));
    assert!(!is_animated(Some("cga")));
    assert!(!is_animated(None));
}
