// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;

use super::{HardpointKind, InteriorFit};

/// How far in metres a room model of the root Entity may protrude beyond its model's box.
///
/// Every root container of the real ships measured lies inside the box; a cockpit container
/// moved 3 m along the hull protrudes by about 3 m.
pub const HULL_SLACK: f32 = 1.0;

/// A rule a correct placement of the containers breaks. The fields named `controller`, `seat` and
/// `grid` index [`InteriorFit::hardpoints`]; `container` indexes [`InteriorFit::containers`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Misfit {
    /// The flight controller of the root vehicle and its pilot seat lie in no common container.
    CockpitSplit {
        /// The controller, an index into the hardpoints.
        controller: usize,
        /// Its pilot seat, an index into the hardpoints.
        seat: usize,
    },
    /// A cargo grid lies in no container that is free of cockpit hardpoints, so no room of its
    /// own holds it.
    CargoOutsideCargoRoom {
        /// The cargo grid, an index into the hardpoints.
        grid: usize,
    },
    /// A room model of a container protrudes beyond the box of the root Entity's model by more
    /// than [`HULL_SLACK`].
    RoomOutsideHull {
        /// The container, an index into the containers.
        container: usize,
    },
}

/// The index of the root Entity's part in the Assembly.
const ROOT_PART: usize = 0;

impl InteriorFit {
    /// Returns the rules the placement breaks, `tolerance` metres being how far a hardpoint may
    /// lie outside a model box and still count as in it:
    ///
    /// - the flight controller of the root vehicle and its pilot seat lie in the same container;
    ///   a docked ship's controller sits at that ship's origin and is not checked;
    /// - every cargo grid lies in a container that holds no cockpit hardpoint, that is no flight
    ///   controller and no pilot seat;
    /// - the room models of every container of the root Entity lie within [`HULL_SLACK`] of the
    ///   box of the root Entity's model (containers of attached Entities, such as a docked ship,
    ///   are left out), so a section moved along the hull out of it is caught even when every
    ///   hardpoint still lies in some room.
    ///
    /// A hardpoint lies in a container when it is within `tolerance` of the box of a single static
    /// model of it ([`ContainerFit::models`](super::ContainerFit::models)), not of the union of
    /// all its models. The hull rule uses the container's room models
    /// ([`ContainerFit::rooms`](super::ContainerFit::rooms)).
    #[must_use]
    pub fn misfits(&self, tolerance: f32) -> Vec<Misfit> {
        let pairs = self.pilot_pairs();
        let mut misfits = self.cockpit_splits(&pairs, tolerance);
        misfits.extend(self.cargo_outside(&pairs, tolerance));
        misfits.extend(self.rooms_outside_hull());
        misfits
    }

    fn rooms_outside_hull(&self) -> Vec<Misfit> {
        let Some(hull) = &self.hull else {
            return Vec::new();
        };
        self.containers
            .iter()
            .enumerate()
            .filter(|(_, container)| container.owner == ROOT_PART)
            .filter(|(_, container)| {
                container
                    .overshoot(hull)
                    .is_some_and(|overshoot| overshoot > HULL_SLACK)
            })
            .map(|(container, _)| Misfit::RoomOutsideHull { container })
            .collect()
    }

    /// Returns whether container `container` holds hardpoint `hardpoint` within `tolerance`.
    fn holds(&self, container: usize, hardpoint: usize, tolerance: f32) -> bool {
        self.containers
            .get(container)
            .zip(self.hardpoints.get(hardpoint))
            .is_some_and(|(container, fit)| container.holds(fit.hardpoint.position, tolerance))
    }

    fn cockpit_splits(&self, pairs: &[(usize, usize)], tolerance: f32) -> Vec<Misfit> {
        pairs
            .iter()
            .filter(|&&(controller, _)| {
                self.hardpoints
                    .get(controller)
                    .is_some_and(|fit| fit.hardpoint.vehicle.is_empty())
            })
            .filter(|&&(controller, seat)| {
                !(0..self.containers.len()).any(|container| {
                    self.holds(container, controller, tolerance)
                        && self.holds(container, seat, tolerance)
                })
            })
            .map(|&(controller, seat)| Misfit::CockpitSplit { controller, seat })
            .collect()
    }

    fn cargo_outside(&self, pairs: &[(usize, usize)], tolerance: f32) -> Vec<Misfit> {
        let cockpit: HashSet<usize> = self
            .indexed(HardpointKind::Controller)
            .map(|(index, _)| index)
            .chain(pairs.iter().map(|&(_, seat)| seat))
            .collect();
        let cockpit_containers: HashSet<usize> = (0..self.containers.len())
            .filter(|&container| {
                cockpit
                    .iter()
                    .any(|&hardpoint| self.holds(container, hardpoint, tolerance))
            })
            .collect();
        self.indexed(HardpointKind::CargoGrid)
            .filter(|&(grid, _)| {
                !(0..self.containers.len()).any(|container| {
                    !cockpit_containers.contains(&container)
                        && self.holds(container, grid, tolerance)
                })
            })
            .map(|(grid, _)| Misfit::CargoOutsideCargoRoom { grid })
            .collect()
    }
}
