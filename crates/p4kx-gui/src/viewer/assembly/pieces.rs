// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::{Assembly, INTERIOR_NODE};
use p4kx_extraction::model::{MaterialStatus, Model, TintPalette};

use crate::viewer::scene::Rows;

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
/// each container's root below the root part.
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
    let Some(interior) = assembly.interior() else {
        return pieces;
    };
    let Some(root) = assembly.root() else {
        return pieces;
    };
    for container in interior.containers() {
        let base = pieces.len();
        let anchor = root.world().then(container.placement());
        for part in container.interior().parts() {
            pieces.push(Piece {
                name: format!("{INTERIOR_NODE}/{}", part.name()),
                path: "",
                parent: part.parent().map_or(Some(0), |parent| Some(base + parent)),
                model: part.model(),
                world: anchor.then(part.world()).rows,
                materials: part.materials(),
                palette: None,
                interior: true,
            });
        }
    }
    pieces
}
