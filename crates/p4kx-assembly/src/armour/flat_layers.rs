// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::BTreeSet;

use p4kx_gltf::diffuse_texture_path;
use p4kx_model::{MaterialStatus, Model};

use super::jobs::{Job, issue};
use crate::{Issue, IssueKind};

/// Lists as one issue the layered materials `model` draws that have no diffuse map: a shader such
/// as `LayerBlend_V2` takes its colour from the tint palette and the maps of its layers, which
/// are not drawn, so these materials show the colours of their layers.
pub(super) fn note_flat_layers(
    job: &Job,
    model: &Model,
    materials: &MaterialStatus,
    issues: &mut Vec<Issue>,
) {
    let Some(library) = materials.library() else {
        return;
    };
    let used: BTreeSet<u16> = model
        .lods
        .first()
        .into_iter()
        .flat_map(|lod| &lod.meshes)
        .flat_map(|mesh| &mesh.submeshes)
        .map(|part| part.material_id)
        .collect();
    let count = used
        .into_iter()
        .filter_map(|id| library.material(id))
        .filter(|material| {
            !material.hidden()
                && !material.layers.is_empty()
                && diffuse_texture_path(material).is_none()
        })
        .count();
    if count > 0 {
        issues.push(issue(job, IssueKind::LayersWithoutDiffuse { count }));
    }
}
