// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::sync::Arc;

use p4kx_character::{Dna, DnaMatrix, DnaRig, RigLogic};
use p4kx_model::{Bone, Model};

/// Where the joints of a head rig are among the bones of its skin, and how far the Character's
/// own joints sit from the base head's.
pub(super) struct Facial {
    /// The offset of each bone of the skin, parent-local and in the rig's axes; zero for bones
    /// the rig does not move.
    pub(super) offsets: Vec<[f32; 3]>,
    /// The bone of the skin each joint of the rig is, in the rig's order.
    pub(super) joints: Vec<usize>,
    /// The bones of the skin with the bind poses of the head's own skeleton file, when it has
    /// one: the skeleton the head is appended to takes these instead of the skin's.
    pub(super) posed: Option<Vec<Bone>>,
}

/// Returns the bones of the first level of detail of `model`, those of a skeleton file without one.
pub(super) fn bones_of(model: &Model) -> &[Bone] {
    model
        .lods
        .first()
        .map_or(model.bones.as_slice(), |lod| model.bones_of(lod))
}

/// Returns the bone of the first level of detail of `model` for each joint in `hashes`, `usize::MAX`
/// for a joint the model lacks, and the number of bones.
fn locate(model: &Model, hashes: &[u32]) -> (Vec<usize>, usize) {
    let bones = bones_of(model);
    let mut by_controller: HashMap<u32, usize> = HashMap::with_capacity(bones.len());
    for (index, bone) in bones.iter().enumerate() {
        by_controller.entry(bone.controller_id).or_insert(index);
    }
    let joints = hashes
        .iter()
        .map(|hash| by_controller.get(hash).copied().unwrap_or(usize::MAX))
        .collect();
    (joints, bones.len())
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
    let (joints, bones) = locate(model, hashes);
    let mut offsets = vec![[0.0_f32; 3]; bones];
    for (&bone, offset) in joints.iter().zip(moves) {
        if let Some(slot) = offsets.get_mut(bone) {
            *slot = offset;
        }
    }
    Ok(Facial {
        offsets,
        joints,
        posed: None,
    })
}

/// Matches the joints of a story character's own head rig to the bones of `model`. The joints sit
/// where `posed`, the bones with the poses of the head's skeleton file, puts them, or where the
/// skin does without it, so no bone has an offset.
pub(super) fn own_facial(model: &Model, logic: &RigLogic, posed: Option<Vec<Bone>>) -> Facial {
    let (joints, bones) = locate(model, logic.joint_hashes());
    Facial {
        offsets: vec![[0.0; 3]; bones],
        joints,
        posed,
    }
}

/// Where the rig logic of a head comes from.
#[derive(Debug, Clone)]
pub(crate) enum FaceSource {
    /// A head shaped by a Character's DNA: the DNA rig beside the skin mixes the delta records of
    /// its stock heads, and the `RigLogic` chunk of the skin names the controls.
    Dna { skin: String, rig: String, dna: Dna },
    /// A story character's own head: the `.RigLogic` file holds the names and the delta records.
    Own { logic: String },
}

/// How a head part of an [`Armour`](super::Armour) is tied to the rig that animates it.
#[derive(Debug, Clone)]
pub(crate) struct FaceLink {
    pub(crate) source: FaceSource,
    pub(crate) joints: Vec<usize>,
    pub(crate) blendshapes: Vec<usize>,
}

impl FaceLink {
    pub(crate) fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}
