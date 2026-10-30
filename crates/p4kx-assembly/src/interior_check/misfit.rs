// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;

use super::{HardpointKind, InteriorFit};

/// A rule a correct placement of the containers breaks; the numbers index
/// [`InteriorFit::hardpoints`].
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
}

impl InteriorFit {
    /// Returns the rules the placement breaks, `tolerance` metres being how far a hardpoint may
    /// lie outside a room box and still count as in it:
    ///
    /// - the flight controller of the root vehicle and its pilot seat lie in the same container;
    ///   a docked ship's controller sits at that ship's origin and is not checked;
    /// - every cargo grid lies in a container that holds no cockpit hardpoint, that is no flight
    ///   controller and no pilot seat.
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
        misfits
    }
}
