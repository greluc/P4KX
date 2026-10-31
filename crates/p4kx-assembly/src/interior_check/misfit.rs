// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;

use super::{ContainerFit, HardpointKind, InteriorFit};

/// A rule a correct placement of the containers breaks; the numbers index
/// [`InteriorFit::hardpoints`], except for [`Misfit::RoomsApart`], which indexes
/// [`InteriorFit::containers`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Misfit {
    /// The flight controller of the root vehicle and its pilot seat lie in no common container.
    CockpitSplit {
        /// The controller.
        controller: usize,
        /// Its pilot seat.
        seat: usize,
    },
    /// A cargo grid lies in no container that is free of cockpit hardpoints, so no room of its
    /// own holds it.
    CargoOutsideCargoRoom {
        /// The cargo grid.
        grid: usize,
    },
    /// A seat of the root vehicle lies in no room model of any container.
    SeatOutsideRooms {
        /// The seat.
        seat: usize,
    },
    /// The room models of a container lie farther from those of every other container than a
    /// neighbouring room does in a hull, whose rooms abut one another.
    RoomsApart {
        /// The index into [`InteriorFit::containers`].
        container: usize,
    },
}

impl InteriorFit {
    /// Returns the rules the placement breaks, `tolerance` metres being how far a hardpoint may
    /// lie outside a room box and still count as in it, and two rooms count as abutting:
    ///
    /// - the flight controller of the root vehicle and its pilot seat lie in the same container;
    ///   a docked ship's controller sits at that ship's origin and is not checked;
    /// - every seat of the root vehicle lies in a container; the nearest seat to the controller
    ///   need not be the pilot's, so a cockpit moved along the hull is caught by the seats at
    ///   its far end;
    /// - every cargo grid lies in a container that holds no cockpit hardpoint, that is no flight
    ///   controller and no pilot seat;
    /// - the room models of every container lie within `tolerance` of those of another container,
    ///   so a section moved along the hull away from its neighbours is caught even when every
    ///   hardpoint still lies in some room.
    ///
    /// A hardpoint lies in a container when it is within `tolerance` of the box of a single room
    /// model of it ([`ContainerFit::rooms`]), not of the union of all
    /// its models.
    #[must_use]
    pub fn misfits(&self, tolerance: f32) -> Vec<Misfit> {
        let holds = |container: usize, hardpoint: usize| {
            self.containers
                .get(container)
                .zip(self.hardpoints.get(hardpoint))
                .is_some_and(|(container, fit)| container.holds(fit.hardpoint.position, tolerance))
        };
        let pairs = self.pilot_pairs();
        let mut misfits = Vec::new();
        for &(controller, seat) in &pairs {
            let root = self
                .hardpoints
                .get(controller)
                .is_some_and(|fit| fit.hardpoint.vehicle.is_empty());
            if root
                && !(0..self.containers.len())
                    .any(|container| holds(container, controller) && holds(container, seat))
            {
                misfits.push(Misfit::CockpitSplit { controller, seat });
            }
        }
        for (seat, at) in self.indexed(HardpointKind::Seat) {
            if at.hardpoint.vehicle.is_empty()
                && !(0..self.containers.len()).any(|container| holds(container, seat))
            {
                misfits.push(Misfit::SeatOutsideRooms { seat });
            }
        }
        let cockpit: HashSet<usize> = self
            .indexed(HardpointKind::Controller)
            .map(|(index, _)| index)
            .chain(pairs.iter().map(|&(_, seat)| seat))
            .collect();
        let cockpit_containers: HashSet<usize> = (0..self.containers.len())
            .filter(|&container| cockpit.iter().any(|&hardpoint| holds(container, hardpoint)))
            .collect();
        for (grid, _) in self.indexed(HardpointKind::CargoGrid) {
            let in_cargo_room = (0..self.containers.len()).any(|container| {
                !cockpit_containers.contains(&container) && holds(container, grid)
            });
            if !in_cargo_room {
                misfits.push(Misfit::CargoOutsideCargoRoom { grid });
            }
        }
        misfits.extend(self.containers_apart(tolerance));
        misfits
    }

    /// Returns a [`Misfit::RoomsApart`] for each container with room models that lies more than
    /// `tolerance` metres from every other container with room models.
    fn containers_apart(&self, tolerance: f32) -> Vec<Misfit> {
        let roomed: Vec<(usize, &ContainerFit)> = self
            .containers
            .iter()
            .enumerate()
            .filter(|(_, container)| !container.rooms().is_empty())
            .collect();
        if roomed.len() < 2 {
            return Vec::new();
        }
        roomed
            .iter()
            .filter(|&&(index, container)| {
                roomed
                    .iter()
                    .filter(|&&(other, _)| other != index)
                    .all(|&(_, other)| {
                        container
                            .distance_to(other)
                            .is_none_or(|distance| distance > tolerance)
                    })
            })
            .map(|&(container, _)| Misfit::RoomsApart { container })
            .collect()
    }
}
