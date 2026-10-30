// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::{Assembly, EntityInterior, INTERIOR_NODE};
use p4kx_extraction::model::{MaterialStatus, Model, TintPalette};

use crate::scene::Rows;

#[cfg(test)]
mod tests;

/// One thing the scene places: a part of the Loadout, or a part of the Interior with the
/// transform of its container and the hull applied.
pub(super) struct Piece<'a> {
    pub(super) name: String,
    pub(super) path: &'a str,
    pub(super) parent: Option<usize>,
    pub(super) model: Option<&'a Model>,
    pub(super) world: Rows,
    pub(super) materials: &'a MaterialStatus,
    pub(super) palette: Option<&'a TintPalette>,
    pub(super) interior: bool,
}

/// Lists the pieces of `assembly`: its parts, then the parts of the containers of its Interior,
/// each container's root below the part of the Entity that names it, then the bone attachments of
/// the parts, each below its part.
pub(super) fn pieces_of(assembly: &Assembly) -> Vec<Piece<'_>> {
    let mut pieces: Vec<Piece<'_>> = assembly
        .parts()
        .iter()
        .map(|part| Piece {
            name: part.name().to_owned(),
            path: part.path(),
            parent: part.parent(),
            model: part.model(),
            world: part.world().rows,
            materials: part.materials(),
            palette: part.palette(),
            interior: false,
        })
        .collect();
    if let Some(interior) = assembly.interior() {
        push_containers(&mut pieces, assembly, interior);
    }
    for (index, part) in assembly.parts().iter().enumerate() {
        for attached in part.attachments() {
            pieces.push(Piece {
                name: format!("{}/{}", part.name(), attached.name()),
                path: "",
                parent: Some(index),
                model: Some(attached.model()),
                world: part.world().then(attached.local()).rows,
                materials: attached.materials(),
                palette: part.palette(),
                interior: false,
            });
        }
    }
    pieces
}

fn push_containers<'a>(
    pieces: &mut Vec<Piece<'a>>,
    assembly: &'a Assembly,
    interior: &'a EntityInterior,
) {
    for container in interior.containers() {
        let base = pieces.len();
        let owner = assembly.parts().get(container.part());
        let anchor = owner.map_or_else(
            || *container.placement(),
            |owner| owner.world().then(container.placement()),
        );
        let owner_index = owner.map(|_| container.part());
        let prefix = match owner.map_or("", |owner| owner.path()) {
            "" => INTERIOR_NODE.to_owned(),
            port => format!("{port}/{INTERIOR_NODE}"),
        };
        for part in container.interior().parts() {
            pieces.push(Piece {
                name: format!("{prefix}/{}", part.name()),
                path: "",
                parent: part
                    .parent()
                    .map_or(owner_index, |parent| Some(base + parent)),
                model: part.model(),
                world: anchor.then(part.world()).rows,
                materials: part.materials(),
                palette: part.palette(),
                interior: true,
            });
        }
    }
}
