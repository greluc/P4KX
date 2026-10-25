// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_character::Dna;
use p4kx_model::{Bone, BoneMatch, Model, extend_skeleton};

use super::ArmourPart;
use super::facial::{FaceLink, Facial};
use super::jobs::{Job, issue};
use crate::{Issue, IssueKind};

/// What a shaped head brings to the skeleton: where its rig's joints are among its bones, and the
/// blendshape number of each of its morph targets.
pub(super) struct Face {
    pub(super) facial: Option<Facial>,
    pub(super) rig: String,
    pub(super) numbers: Vec<usize>,
}

/// A head part and what ties it to its rig.
pub(super) struct Found {
    pub(super) part: usize,
    pub(super) skin: String,
    pub(super) dna: Option<Dna>,
    pub(super) face: Face,
}

impl Found {
    /// Returns the part `part` made of the skin at `skin` of a story character's own head: its
    /// bones join the skeleton at the positions its skin gives, and it has no rig to animate.
    pub(super) fn bespoke(part: usize, skin: &str) -> Self {
        Self {
            part,
            skin: skin.to_owned(),
            dna: None,
            face: Face {
                facial: None,
                rig: String::new(),
                numbers: Vec::new(),
            },
        }
    }

    /// Returns the head part `part` made of the skin at `skin` when it was shaped by `dna` and
    /// has a face.
    pub(super) fn of(
        part: usize,
        skin: &str,
        dna: Option<&Dna>,
        face: Option<Face>,
    ) -> Option<Self> {
        Some(Self {
            part,
            skin: skin.to_owned(),
            dna: Some(dna?.clone()),
            face: face?,
        })
    }
}

fn bones_of(model: &Model) -> Vec<Bone> {
    model
        .lods
        .first()
        .map(|lod| model.bones_of(lod).to_vec())
        .unwrap_or_default()
}

/// Appends the bones of each head to the skeleton, moved to the Character's own joint positions
/// where the rig gives them, and ties each head part to its rig.
pub(super) fn extend_with_faces(skeleton: &mut Model, parts: &mut [ArmourPart], found: Vec<Found>) {
    for head in found {
        let Some(part) = parts.get_mut(head.part) else {
            continue;
        };
        let bones = bones_of(&part.model);
        let moves = head
            .face
            .facial
            .as_ref()
            .map(|facial| facial.offsets.as_slice())
            .unwrap_or_default();
        skeleton.bones = extend_skeleton(&skeleton.bones, &bones, moves);
        let (Some(facial), Some(dna)) = (head.face.facial, head.dna) else {
            continue;
        };
        let mut by_name: HashMap<String, usize> = HashMap::with_capacity(skeleton.bones.len());
        for (index, bone) in skeleton.bones.iter().enumerate() {
            by_name
                .entry(bone.name.to_ascii_lowercase())
                .or_insert(index);
        }
        let joints = facial
            .joints
            .iter()
            .map(|&joint| {
                bones
                    .get(joint)
                    .and_then(|bone| by_name.get(&bone.name.to_ascii_lowercase()))
                    .copied()
                    .unwrap_or(usize::MAX)
            })
            .collect();
        part.face = Some(
            FaceLink {
                skin: head.skin,
                rig: head.face.rig,
                dna,
                joints,
                blendshapes: head.face.numbers,
            }
            .shared(),
        );
    }
}

/// Returns `skeleton` with the bones of the heads appended and notes how each part fits it.
pub(super) fn settled(
    mut skeleton: Model,
    parts: &mut [ArmourPart],
    heads: Vec<Found>,
    jobs: &[&Job],
    issues: &mut Vec<Issue>,
) -> Model {
    extend_with_faces(&mut skeleton, parts, heads);
    for (job, part) in jobs.iter().zip(parts.iter()) {
        note_fit(job, part, &skeleton, issues);
    }
    skeleton
}

/// Notes how a part fits the skeleton: bones it lacks, and a model that is not skinned at all.
fn note_fit(job: &Job, part: &ArmourPart, skeleton: &Model, issues: &mut Vec<Issue>) {
    let model = &part.model;
    let Some(lod) = model.lods.first() else {
        return;
    };
    let skinned = lod.meshes.iter().any(|mesh| mesh.influences.is_some());
    if !skinned {
        issues.push(issue(job, IssueKind::NotSkinned));
        return;
    }
    let matched = BoneMatch::new(&skeleton.bones, model.bones_of(lod));
    let count = matched.unmatched().len();
    if count > 0 {
        issues.push(issue(job, IssueKind::BonesNotInSkeleton { count }));
    }
}
