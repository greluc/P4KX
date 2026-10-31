// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::sync::Arc;

use p4kx_character::{Dna, DnaMatrix, DnaRig, RigLogic};
use p4kx_model::Model;

/// Where the joints of a head rig are among the bones of its skin, and how far the Character's
/// own joints sit from the base head's.
pub(super) struct Facial {
    /// The offset of each bone of the skin, parent-local and in the rig's axes; zero for bones
    /// the rig does not move.
    pub(super) offsets: Vec<[f32; 3]>,
    /// The bone of the skin each joint of the rig is, in the rig's order.
    pub(super) joints: Vec<usize>,
    /// The rig logic of the skin, which names the controls and blendshapes.
    pub(super) logic: RigLogic,
}

/// Matches the joints of the rig, which `logic` lists by the CRC-32 of their names, to the bones of
/// `model`, and gets the Character's joint offsets from the rig.
pub(super) fn facial(
    model: &Model,
    logic: &RigLogic,
    rig: &DnaRig<'_>,
    matrix: &DnaMatrix,
) -> Result<Facial, String> {
    let moves = rig
        .neutral_joint_offsets(matrix)
        .map_err(|error| error.to_string())?;
    let hashes = logic.joint_hashes();
    if hashes.len() != moves.len() {
        return Err(format!(
            "the skin lists {} joints, the DNA rig has {}",
            hashes.len(),
            moves.len()
        ));
    }
    let bones = model
        .lods
        .first()
        .map_or(model.bones.as_slice(), |lod| model.bones_of(lod));
    let mut by_controller: HashMap<u32, usize> = HashMap::with_capacity(bones.len());
    for (index, bone) in bones.iter().enumerate() {
        by_controller.entry(bone.controller_id).or_insert(index);
    }
    let mut offsets = vec![[0.0_f32; 3]; bones.len()];
    let mut joints = Vec::with_capacity(hashes.len());
    for (hash, offset) in hashes.iter().zip(moves) {
        let bone = by_controller.get(hash).copied();
        if let Some(slot) = bone.and_then(|bone| offsets.get_mut(bone)) {
            *slot = offset;
        }
        joints.push(bone.unwrap_or(usize::MAX));
    }
    Ok(Facial {
        offsets,
        joints,
        logic: logic.clone(),
    })
}

/// How a head part of an [`Armour`](super::Armour) is tied to the rig that animates it.
#[derive(Debug, Clone)]
pub(crate) struct FaceLink {
    pub(crate) rig: String,
    pub(crate) dna: Dna,
    pub(crate) joints: Vec<usize>,
    pub(crate) blendshapes: Vec<usize>,
    /// The rig logic of the head skin, read once when the head is shaped.
    pub(crate) logic: RigLogic,
}

impl FaceLink {
    pub(crate) fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}
