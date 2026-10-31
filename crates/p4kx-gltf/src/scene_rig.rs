// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{Bone, Influences};

use crate::glb_plan::Skeletal;
use crate::instance::Instance;
use crate::rig::remap;
use crate::skin::inverse_binds;

/// The skeleton the skinned models of a scene are bound to: its inverse bind matrices and, per
/// model instance and mesh, the influences with the bones of the skeleton.
pub(crate) struct SceneRig<'m> {
    pub(crate) bones: &'m [Bone],
    pub(crate) binds: Vec<[f32; 16]>,
    pub(crate) influences: Vec<Vec<Vec<Influences>>>,
}

impl<'m> SceneRig<'m> {
    /// Binds the skinned meshes of `instances` to `skeleton` by bone name; `None` when the
    /// skeleton is empty or no instance has a skinned mesh at its level of detail.
    ///
    /// An instance that reuses the meshes of a twin is not remapped.
    pub(crate) fn new(skeleton: &'m [Bone], instances: &[Instance<'_>]) -> Option<Self> {
        if skeleton.is_empty() || !instances.iter().any(is_skinned) {
            return None;
        }
        let influences = instances
            .iter()
            .map(|instance| {
                if instance.twin.is_some() {
                    Vec::new()
                } else {
                    remap(skeleton, instance)
                }
            })
            .collect();
        Some(Self {
            bones: skeleton,
            binds: inverse_binds(skeleton),
            influences,
        })
    }
}

impl SceneRig<'_> {
    /// Returns what the layout needs to weight the skinned meshes to the skeleton.
    pub(crate) fn skeletal(&self) -> Skeletal<'_> {
        Skeletal {
            binds: &self.binds,
            influences: Some(&self.influences),
        }
    }
}

fn is_skinned(instance: &Instance<'_>) -> bool {
    instance
        .lod
        .is_some_and(|lod| lod.meshes.iter().any(|mesh| mesh.influences.is_some()))
}
