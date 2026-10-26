// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::{Assembly, Placement};
use serde::Serialize;

/// How one part of the Assembly was placed, as `entity glb --json` reports it.
#[derive(Serialize)]
pub(crate) struct PlacementJson<'a> {
    path: &'a str,
    placement: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    helper: Option<&'a str>,
    modelled: bool,
    hidden: bool,
}

/// Returns the placement of every part, the root first.
pub(crate) fn placements_json(assembly: &Assembly) -> Vec<PlacementJson<'_>> {
    assembly
        .parts()
        .iter()
        .map(|part| PlacementJson {
            path: part.path(),
            placement: part.placement().code(),
            helper: match part.placement() {
                Placement::Helper(helper) => Some(helper),
                _ => None,
            },
            modelled: part.model().is_some(),
            hidden: part.is_hidden(),
        })
        .collect()
}
