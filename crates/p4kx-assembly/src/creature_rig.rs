// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The skeleton the skins of a creature Entity are bound to: the bones of the root Entity's own
//! model, when its geometry is a character file outside the human rigs.

use p4kx_model::{Bone, BoneMatch, Model};

use crate::geometry::is_rigged_path_outside_human_rigs;
use crate::{Issue, IssueKind, Part};

/// Returns the bones of the root part's model at the level `level` of detail, or `None` when the
/// root's geometry is no character file outside the human rigs, its skeleton file is missing or
/// unreadable, or its model has no bones.
pub(crate) fn skeleton_of<'a>(
    parts: &'a [Part],
    issues: &[Issue],
    level: u32,
) -> Option<&'a [Bone]> {
    let root = parts.first()?;
    if !root
        .geometry
        .as_deref()
        .is_some_and(is_rigged_path_outside_human_rigs)
        || issues.iter().any(|issue| {
            issue.path.is_empty()
                && matches!(
                    issue.kind,
                    IssueKind::SkeletonMissing { .. } | IssueKind::SkeletonUnreadable { .. }
                )
        })
    {
        return None;
    }
    let model = root.model.as_deref()?;
    let bones = model.bones_of(model.nearest_lod(level)?);
    (!bones.is_empty()).then_some(bones)
}

/// Lists, per skinned part below the root, the bones of its model that the root's skeleton lacks;
/// their vertices follow the nearest parent bone the skeleton has.
pub(crate) fn note_fit(parts: &[Part], level: u32, issues: &mut Vec<Issue>) {
    let Some(skeleton) = skeleton_of(parts, issues, level) else {
        return;
    };
    let notes: Vec<Issue> = parts
        .iter()
        .skip(1)
        .filter_map(|part| {
            let model = part
                .model
                .as_deref()
                .filter(|model| is_skinned(model, level))?;
            let lod = model.nearest_lod(level)?;
            let count = BoneMatch::new(skeleton, model.bones_of(lod))
                .unmatched()
                .len();
            (count > 0).then(|| Issue {
                path: part.path.clone(),
                name: part.class_name.clone(),
                kind: IssueKind::BonesNotInSkeleton { count },
            })
        })
        .collect();
    issues.extend(notes);
}

fn is_skinned(model: &Model, level: u32) -> bool {
    model
        .nearest_lod(level)
        .is_some_and(|lod| lod.meshes.iter().any(|mesh| mesh.influences.is_some()))
}
