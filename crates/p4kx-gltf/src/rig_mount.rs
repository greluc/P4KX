// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{Bone, Transform};

use crate::axes::{Trs, decompose};
use crate::nodes::{RigModel, placement, worn_skin};

/// The bone of the skeleton an unskinned piece is attached to and where the piece sits below it.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct RigMount {
    /// The index of the bone in the skeleton.
    pub bone: usize,
    /// The transform from the piece's space to the bone's, in the skeleton's axes.
    pub transform: Transform,
}

impl RigMount {
    /// Attaches a piece to the bone `bone`, placed by `transform` in the bone's space.
    #[must_use]
    pub const fn new(bone: usize, transform: Transform) -> Self {
        Self { bone, transform }
    }
}

/// Returns the mount of `rig` when the skeleton written has its bone.
pub(crate) fn mount_of(rig: &RigModel<'_>, bones: &[Bone]) -> Option<RigMount> {
    rig.mount.filter(|mount| mount.bone < bones.len())
}

/// Returns the transform of the unskinned node at `position` of `rig`: the transforms of the
/// skinned nodes it hangs past, the mount when the node is a root of a mounted model, and its own.
pub(crate) fn unskinned_trs(
    rig: &RigModel<'_>,
    position: usize,
    mount: Option<RigMount>,
    has_skin: bool,
) -> Trs {
    let placed = placement(&rig.model.nodes, position, |at| {
        worn_skin(rig, at, has_skin)
    });
    let carried = match (placed.parent, mount) {
        (None, Some(mount)) => mount.transform.then(&placed.carried),
        _ => placed.carried,
    };
    let own = rig
        .model
        .nodes
        .get(position)
        .map_or(Transform::IDENTITY, |node| node.transform);
    decompose(&carried.then(&own))
}
