// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Checks that each container of an Interior holds the hardpoints it serves.
//!
//! [`InteriorFit::new`] measures every container of an [`EntityInterior`] in the world: the box
//! around the static models it places and the box of each room model among them (a hardpoint is
//! held by a single room model, not by the union), the functional hardpoints of the Assembly (controller,
//! seat, cargo grid, door; told by the item type of the Entity at the port, never by name) inside
//! and outside it, and how far its animated parts lie from it. [`InteriorFit::misfits`] applies
//! the rules a correct placement keeps.

mod bounds;
mod fit;
mod hardpoint;
mod misfit;

pub use bounds::WorldBox;
pub use fit::{AnimatedPart, ContainerFit, HardpointFit, ROOM_VOLUME_SHARE};
pub use hardpoint::{Hardpoint, HardpointKind};
pub use misfit::Misfit;

use p4kx_model::Transform;

use crate::{Assembly, EntityInterior, EntityNode};

/// The containers of an [`EntityInterior`] measured against the hardpoints of its [`Assembly`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InteriorFit {
    containers: Vec<ContainerFit>,
    hardpoints: Vec<HardpointFit>,
    unplaced: Vec<String>,
}

impl InteriorFit {
    /// Measures `interior`, built for `assembly` from the Entity `tree`.
    ///
    /// A container sits at its owner part's world transform composed with its placement. A
    /// container whose owner part is missing from the Assembly cannot be placed: it is left out
    /// of [`Self::containers`] and listed in [`Self::unplaced`].
    #[must_use]
    pub fn new(assembly: &Assembly, tree: &EntityNode, interior: &EntityInterior) -> Self {
        Self::new_displaced(assembly, tree, interior, None)
    }

    /// Measures `interior` like [`Self::new`], with container `displaced.0` of
    /// [`EntityInterior::containers`] moved by `displaced.1` metres in the world first: the check
    /// of a misplaced container.
    #[must_use]
    pub fn new_displaced(
        assembly: &Assembly,
        tree: &EntityNode,
        interior: &EntityInterior,
        displaced: Option<(usize, [f32; 3])>,
    ) -> Self {
        let hardpoints = hardpoint::collect(assembly, tree);
        let mut containers = Vec::new();
        let mut unplaced = Vec::new();
        for (index, container) in interior.containers().iter().enumerate() {
            match assembly.parts().get(container.part()) {
                Some(owner) => {
                    let mut world = *owner.world();
                    if let Some((_, [x, y, z])) = displaced.filter(|(moved, _)| *moved == index) {
                        world = Transform {
                            rows: [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]],
                        }
                        .then(&world);
                    }
                    containers.push(ContainerFit::measure(&world, container, &hardpoints));
                }
                None => unplaced.push(container.reference().file().to_owned()),
            }
        }
        Self::from_measured(containers, hardpoints, unplaced)
    }

    pub(crate) fn from_measured(
        containers: Vec<ContainerFit>,
        hardpoints: Vec<Hardpoint>,
        unplaced: Vec<String>,
    ) -> Self {
        let hardpoints = hardpoints
            .into_iter()
            .map(|hardpoint| HardpointFit::measure(hardpoint, &containers))
            .collect();
        Self {
            containers,
            hardpoints,
            unplaced,
        }
    }

    /// Returns the measured containers, in the order of [`EntityInterior::containers`] without
    /// the unplaced ones.
    #[must_use]
    pub fn containers(&self) -> &[ContainerFit] {
        &self.containers
    }

    /// Returns the files of the containers that could not be measured because the part that owns
    /// them is not in the Assembly.
    #[must_use]
    pub fn unplaced(&self) -> &[String] {
        &self.unplaced
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
        self.pilot_pairs()
            .into_iter()
            .filter_map(|(_, seat)| self.hardpoints.get(seat))
            .collect()
    }

    /// Returns each controller with its pilot seat, as indices into [`Self::hardpoints`]; a seat
    /// near several controllers is listed with the first.
    pub(crate) fn pilot_pairs(&self) -> Vec<(usize, usize)> {
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        for (controller, at) in self.indexed(HardpointKind::Controller) {
            let nearest = self
                .indexed(HardpointKind::Seat)
                .filter(|(_, seat)| seat.hardpoint.vehicle == at.hardpoint.vehicle)
                .filter_map(|(index, seat)| {
                    Some((
                        index,
                        squared_distance(at.hardpoint.position, seat.hardpoint.position)?,
                    ))
                })
                .min_by(|(_, a), (_, b)| a.total_cmp(b));
            if let Some((seat, _)) = nearest
                && !pairs.iter().any(|(_, known)| *known == seat)
            {
                pairs.push((controller, seat));
            }
        }
        pairs
    }

    fn indexed(&self, kind: HardpointKind) -> impl Iterator<Item = (usize, &HardpointFit)> {
        self.hardpoints
            .iter()
            .enumerate()
            .filter(move |(_, fit)| fit.hardpoint.kind == kind)
    }
}

fn squared_distance(a: [f32; 3], b: [f32; 3]) -> Option<f32> {
    let squared: f32 = a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum();
    squared.is_finite().then_some(squared)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_measure;
#[cfg(test)]
mod tests_misfit;
