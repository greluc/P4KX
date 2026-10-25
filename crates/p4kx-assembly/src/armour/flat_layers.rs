// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::BTreeSet;

use p4kx_gltf::{diffuse_texture_path, draws_layer_diffuse};

use super::ArmourPart;
use crate::{Issue, IssueKind};

/// Returns the issue that lists the layered materials `part` draws that have no diffuse map: a
/// shader such as `LayerBlend_V2` takes its colour from the tint palette and the maps of its
/// layers; those that no layer gives a diffuse map are drawn in the colours of their layers.
pub(super) fn flat_layers(part: &ArmourPart) -> Option<Issue> {
    let library = part.materials.library()?;
    let used: BTreeSet<u16> = part
        .model
        .lods
        .first()
        .into_iter()
        .flat_map(|lod| &lod.meshes)
        .flat_map(|mesh| &mesh.submeshes)
        .map(|submesh| submesh.material_id)
        .collect();
    let count = used
        .into_iter()
        .filter_map(|id| library.material(id))
        .filter(|material| {
            !material.hidden()
                && !material.layers.is_empty()
                && diffuse_texture_path(material).is_none()
                && !draws_layer_diffuse(material)
        })
        .count();
    (count > 0).then(|| Issue {
        path: String::new(),
        name: part.name.clone(),
        kind: IssueKind::LayersWithoutDiffuse(count),
    })
}
