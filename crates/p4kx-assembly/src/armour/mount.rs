// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gltf::RigMount;
use p4kx_model::{Bone, BoneMatch, BoneTransform, Model, Transform, chained_world_poses};

use super::ArmourPart;
use super::jobs::{Job, issue};
use super::piece::Mount;
use crate::placement::{node_transform, offset};
use crate::{Issue, IssueKind};

/// The turn about X in degrees from the frame of a bone, which has Z up, to the frame of the
/// helper of an item, which has Y up (ADR-0231).
const BONE_TO_HELPER: [f32; 3] = [90.0, 0.0, 0.0];

/// Returns whether the model has a mesh at the level `level` of detail and none of its meshes is
/// skinned.
pub(super) fn is_unskinned(model: &Model, level: u32) -> bool {
    model
        .nearest_lod(level)
        .is_some_and(|lod| lod.meshes.iter().all(|mesh| mesh.influences.is_none()))
}

/// Hangs the unskinned parts that have a mount on the bone of the skeleton their port is
/// attached to; a helper that is no bone of its holder is listed and leaves the part at the origin.
pub(super) fn place(
    jobs: &[&Job],
    parts: &mut [ArmourPart],
    skeleton: &Model,
    level: u32,
    issues: &mut Vec<Issue>,
) {
    for (index, job) in jobs.iter().enumerate() {
        let Some(mount) = &job.mount else { continue };
        if !parts
            .get(index)
            .is_some_and(|part| is_unskinned(&part.model, level))
        {
            continue;
        }
        match mounted(parts, index, mount, skeleton, level) {
            Ok(placed) => {
                if let Some(part) = parts.get_mut(index) {
                    part.mount = Some(placed);
                }
            }
            Err(helper) => issues.push(issue(job, IssueKind::HelperNotFound { helper })),
        }
    }
}

/// Returns the bones of `model` at the level `level` of detail.
fn bones_at(model: &Model, level: u32) -> &[Bone] {
    model
        .nearest_lod(level)
        .map_or(&[], |lod| model.bones_of(lod))
}

/// Returns the bones that hold the helper of `mount`: those of the body, or of the piece it names,
/// none when that piece is not worn.
fn holder_bones<'a>(
    parts: &'a [ArmourPart],
    mount: &Mount,
    skeleton: &'a Model,
    level: u32,
) -> &'a [Bone] {
    let Some(name) = &mount.holder else {
        return &skeleton.bones;
    };
    parts
        .iter()
        .find(|part| part.name == *name)
        .map_or(&[], |part| bones_at(&part.model, level))
}

/// Returns the bone of the skeleton the part `index` hangs on and where it sits below it: the
/// bone of the holder that the port names, or the skeleton's nearest bone above it, and the
/// offset of the port and of the part's own helper.
///
/// # Errors
///
/// Returns the name of the helper when the holder has no such bone or it cannot be placed.
fn mounted(
    parts: &[ArmourPart],
    index: usize,
    mount: &Mount,
    skeleton: &Model,
    level: u32,
) -> Result<RigMount, String> {
    let missing = || mount.helper.clone();
    let held = holder_bones(parts, mount, skeleton, level);
    let at = held
        .iter()
        .position(|bone| bone.name.eq_ignore_ascii_case(&mount.helper))
        .ok_or_else(missing)?;
    let bone = usize::from(
        BoneMatch::new(&skeleton.bones, held).target(u16::try_from(at).map_err(|_| missing())?),
    );
    let held_pose = chained_world_poses(held).get(at).and_then(transform_of);
    let anchor_pose = chained_world_poses(&skeleton.bones)
        .get(bone)
        .and_then(transform_of)
        .and_then(|pose| pose.inverse());
    let (Some(held_pose), Some(anchor)) = (held_pose, anchor_pose) else {
        return Err(missing());
    };
    let own = parts.get(index).and_then(|part| {
        let name = mount.offset_helper.as_str();
        (!name.is_empty())
            .then(|| node_transform(&part.model, name)?.inverse())
            .flatten()
    });
    let at_port = anchor
        .then(&held_pose)
        .then(&offset([0.0; 3], BONE_TO_HELPER))
        .then(&offset(mount.position(), mount.rotation()));
    Ok(RigMount::new(
        bone,
        own.map_or(at_port, |inner| at_port.then(&inner)),
    ))
}

/// Returns the transform a bone pose stands for.
fn transform_of(pose: &BoneTransform) -> Option<Transform> {
    let [x, y, z, w] = pose.rotation.map(f64::from);
    Transform::from_quaternion([w, x, y, z], pose.translation.map(f64::from))
}
