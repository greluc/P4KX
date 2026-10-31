// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::{HardpointKind, InteriorFit};

/// How far in metres a room model of the root Entity may protrude beyond its model's box.
///
/// Every root container of the real ships measured lies inside the box; a cockpit container
/// moved 3 m along the hull protrudes by about 3 m.
pub const HULL_SLACK: f32 = 1.0;

/// A rule a correct placement of the containers breaks. The fields named `controller` and `grid`
/// index [`InteriorFit::hardpoints`]; `container` indexes [`InteriorFit::containers`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Misfit {
    /// The flight controller of the root vehicle lies farther than the tolerance from every static
    /// model of every container.
    ControllerOutsideRooms {
        /// The controller, an index into the hardpoints.
        controller: usize,
    },
    /// A cargo grid of the root vehicle lies farther than the tolerance from every static model
    /// although another grid of it lies inside one.
    CargoGridOutsideRooms {
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
    /// - the flight controller of the root vehicle lies within `tolerance` of a static model; a
    ///   docked ship's controller sits at that ship's origin and is not checked;
    /// - when some cargo grid of the root vehicle lies inside a static model, every one lies within
    ///   `tolerance` of one: a ship none of whose grids lies inside carries its cargo outside any
    ///   room (the open struts of the Hull C, 0.96 m or more from every model) and is not checked,
    ///   whatever the tolerance;
    /// - the room models of every container of the root Entity lie within [`HULL_SLACK`] of the
    ///   box of the root Entity's model (containers of attached Entities, such as a docked ship,
    ///   are left out), so a section moved along the hull out of it is caught even when every
    ///   hardpoint still lies in some room.
    ///
    /// No rule names a pilot seat, a cockpit container or a cargo container: the census of real
    /// ships found none of them identifiable by structure (ADR-0086).
    ///
    /// A hardpoint lies in a container when it is within `tolerance` of the box of a single static
    /// model of it ([`ContainerFit::models`](super::ContainerFit::models)), not of the union of
    /// all its models. The hull rule uses the container's room models
    /// ([`ContainerFit::rooms`](super::ContainerFit::rooms)).
    #[must_use]
    pub fn misfits(&self, tolerance: f32) -> Vec<Misfit> {
        let mut misfits = self.controllers_outside(tolerance);
        misfits.extend(self.cargo_grids_outside(tolerance));
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

    fn controllers_outside(&self, tolerance: f32) -> Vec<Misfit> {
        self.indexed(HardpointKind::Controller)
            .filter(|(_, fit)| fit.hardpoint.vehicle.is_empty() && !fit.is_within(tolerance))
            .map(|(controller, _)| Misfit::ControllerOutsideRooms { controller })
            .collect()
    }

    /// The cargo grids of the root vehicle farther than `tolerance` from every static model, when
    /// at least one of them lies inside one. Whether the ship is checked at all does not depend on
    /// `tolerance`.
    fn cargo_grids_outside(&self, tolerance: f32) -> Vec<Misfit> {
        let root_grids = || {
            self.indexed(HardpointKind::CargoGrid)
                .filter(|(_, fit)| fit.hardpoint.vehicle.is_empty())
        };
        if !root_grids().any(|(_, fit)| fit.is_inside()) {
            return Vec::new();
        }
        root_grids()
            .filter(|(_, fit)| !fit.is_within(tolerance))
            .map(|(grid, _)| Misfit::CargoGridOutsideRooms { grid })
            .collect()
    }
}
