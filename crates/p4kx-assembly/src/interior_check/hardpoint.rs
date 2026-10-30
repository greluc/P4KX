// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use crate::{Assembly, Attachment, EntityNode};

/// What a hardpoint of a vehicle does, told by the item type of the Entity at its port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HardpointKind {
    /// A flight controller: the place the pilot flies from.
    Controller,
    /// A seat.
    Seat,
    /// A cargo grid: the space cargo is stored in.
    CargoGrid,
    /// A door, ramp or docking hatch: animated geometry that closes a room; the item types `Door`
    /// and `DockingAnimator`.
    Door,
}

impl HardpointKind {
    /// Returns the stable code of the kind, e.g. `cargo_grid`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Controller => "controller",
            Self::Seat => "seat",
            Self::CargoGrid => "cargo_grid",
            Self::Door => "door",
        }
    }

    /// Returns the kind of an Entity with the item type `item_type` (`AttachDef.Type`), or `None`
    /// for an item that is no hardpoint of interest.
    #[must_use]
    pub fn of_item_type(item_type: &str) -> Option<Self> {
        match item_type {
            "FlightController" => Some(Self::Controller),
            "Seat" => Some(Self::Seat),
            "CargoGrid" => Some(Self::CargoGrid),
            "Door" | "DockingAnimator" => Some(Self::Door),
            _ => None,
        }
    }
}

/// One functional hardpoint of an Assembly: where an Entity of a [`HardpointKind`] sits.
#[derive(Debug, Clone, PartialEq)]
pub struct Hardpoint {
    pub(crate) path: String,
    pub(crate) class_name: String,
    pub(crate) kind: HardpointKind,
    pub(crate) position: [f32; 3],
    pub(crate) vehicle: String,
}

impl Hardpoint {
    /// Returns the port path of the Entity, as [`Part::path`](crate::Part::path) spells it.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the class name of the Entity.
    #[must_use]
    pub fn class_name(&self) -> &str {
        &self.class_name
    }

    /// Returns what the hardpoint does.
    #[must_use]
    pub const fn kind(&self) -> HardpointKind {
        self.kind
    }

    /// Returns the world position of the Entity's origin, in metres.
    #[must_use]
    pub const fn position(&self) -> [f32; 3] {
        self.position
    }

    /// Returns the port path of the vehicle the hardpoint belongs to: the nearest Entity above it
    /// with a vehicle implementation, e.g. a docked ship; empty for the Assembly's root Entity.
    #[must_use]
    pub fn vehicle(&self) -> &str {
        &self.vehicle
    }
}

/// Returns the port path of the nearest Entity at or above `path` that has a vehicle
/// implementation, or the empty path of the root.
fn owning_vehicle<'a>(path: &'a str, entities: &HashMap<&str, &EntityNode>) -> &'a str {
    let mut prefix = path;
    loop {
        if entities
            .get(prefix)
            .is_some_and(|entity| entity.vehicle().is_some())
        {
            return prefix;
        }
        match prefix.rsplit_once('/') {
            Some((parent, _)) => prefix = parent,
            None => return "",
        }
    }
}

/// Returns the hardpoints of `assembly`, in the order of its parts: every part whose Entity in
/// `tree` has the item type of a [`HardpointKind`]. A part that `tree` does not hold is skipped.
pub(super) fn collect(assembly: &Assembly, tree: &EntityNode) -> Vec<Hardpoint> {
    let walked = tree.walk();
    let entities: HashMap<&str, &EntityNode> = walked
        .iter()
        .filter_map(|(path, port)| match port.attachment() {
            Attachment::Entity(entity) => Some((path.as_str(), &**entity)),
            _ => None,
        })
        .collect();
    assembly
        .parts()
        .iter()
        .filter_map(|part| {
            let entity = entities.get(part.path())?;
            Some(Hardpoint {
                path: part.path().to_owned(),
                class_name: entity.class_name().to_owned(),
                kind: HardpointKind::of_item_type(entity.attach_type())?,
                position: part.world().translation(),
                vehicle: owning_vehicle(part.path(), &entities).to_owned(),
            })
        })
        .collect()
}
