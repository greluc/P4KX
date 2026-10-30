// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Checks that each container of an Interior holds the hardpoints it serves.
//!
//! [`InteriorFit::new`] measures every container of an [`EntityInterior`] in the world: the box
//! around its static models, the functional hardpoints of the Assembly (controller, seat, cargo
//! grid, door; told by the item type of the Entity at the port, never by name) inside and outside
//! it, and how far its animated parts lie from it.

mod bounds;
mod fit;
mod hardpoint;

pub use bounds::WorldBox;
pub use fit::{AnimatedPart, ContainerFit, HardpointFit};
pub use hardpoint::{Hardpoint, HardpointKind};

use crate::{Assembly, EntityInterior, EntityNode};
use p4kx_model::Transform;

/// The containers of an [`EntityInterior`] measured against the hardpoints of its [`Assembly`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InteriorFit {
    containers: Vec<ContainerFit>,
    hardpoints: Vec<HardpointFit>,
}

impl InteriorFit {
    /// Measures `interior`, built for `assembly` from the Entity `tree`.
    ///
    /// A container sits at its owner part's world transform composed with its placement; a
    /// container whose owner part is missing from the Assembly is measured as if the owner were at
    /// the origin.
    #[must_use]
    pub fn new(assembly: &Assembly, tree: &EntityNode, interior: &EntityInterior) -> Self {
        let hardpoints = hardpoint::collect(assembly, tree);
        let containers: Vec<ContainerFit> = interior
            .containers()
            .iter()
            .map(|container| {
                let owner = assembly
                    .parts()
                    .get(container.part())
                    .map_or(Transform::IDENTITY, |part| *part.world());
                ContainerFit::measure(&owner, container, &hardpoints)
            })
            .collect();
        let hardpoints = hardpoints
            .into_iter()
            .map(|hardpoint| HardpointFit::measure(hardpoint, &containers))
            .collect();
        Self {
            containers,
            hardpoints,
        }
    }

    /// Returns the measured containers, in the order of [`EntityInterior::containers`].
    #[must_use]
    pub fn containers(&self) -> &[ContainerFit] {
        &self.containers
    }

    /// Returns the hardpoints with the room box nearest to each, in the order of the Assembly's
    /// parts.
    #[must_use]
    pub fn hardpoints(&self) -> &[HardpointFit] {
        &self.hardpoints
    }

    /// Returns the hardpoints of `kind`.
    pub fn of_kind(&self, kind: HardpointKind) -> impl Iterator<Item = &HardpointFit> {
        self.hardpoints
            .iter()
            .filter(move |fit| fit.hardpoint.kind == kind)
    }

    /// Returns the pilot seats: for each controller, the seat of the same vehicle nearest to it,
    /// once each. A controller with no such seat at a finite position has none.
    #[must_use]
    pub fn pilot_seats(&self) -> Vec<&HardpointFit> {
        let mut seats: Vec<&HardpointFit> = Vec::new();
        for controller in self.of_kind(HardpointKind::Controller) {
            let at = controller.hardpoint.position;
            let nearest = self
                .of_kind(HardpointKind::Seat)
                .filter(|seat| seat.hardpoint.vehicle == controller.hardpoint.vehicle)
                .filter_map(|seat| Some((seat, squared_distance(at, seat.hardpoint.position)?)))
                .min_by(|(_, a), (_, b)| a.total_cmp(b));
            if let Some((seat, _)) = nearest
                && !seats.iter().any(|known| std::ptr::eq(*known, seat))
            {
                seats.push(seat);
            }
        }
        seats
    }
}

fn squared_distance(a: [f32; 3], b: [f32; 3]) -> Option<f32> {
    let squared: f32 = a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum();
    squared.is_finite().then_some(squared)
}

#[cfg(test)]
mod tests;
