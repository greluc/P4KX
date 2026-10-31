// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The skeleton the skins of a creature Entity are bound to: the bones of the root Entity's own
//! model, when its geometry is a character file outside the human rigs.

use std::sync::Arc;

use p4kx_model::{Bone, BoneMatch, Model, Transform};

use crate::geometry::is_creature_path;
use crate::placement::bone_pose;
use crate::{EntityNode, Issue, IssueKind, LoadoutError, Part, Placement};

/// Returns the bones of the root part's model at the level `level` of detail, or `None` when the
/// root's geometry is no creature file (a character file below `Objects/Characters` outside the
/// human rigs), its skeleton file is missing or unreadable, or its model has no bones.
///
/// The bones are in the root's space, which is the Assembly's: the root sits at the identity and
/// a skin part is placed at the identity too (ADR-0224), so the skins agree with the bones. A
/// skinned mesh ignores the transform of its node in glTF, so the port offset of a skin part has
/// no effect.
pub(crate) fn skeleton_of<'a>(
    parts: &'a [Part],
    issues: &[Issue],
    level: u32,
) -> Option<&'a [Bone]> {
    let model = rig_model(parts, issues)?;
    let bones = model.bones_of(model.nearest_lod(level)?);
    (!bones.is_empty()).then_some(bones)
}

/// Returns the model of the root part when it is a creature whose skeleton file is present, the
/// model whose bones the skins and the item ports of the skins are bound to.
fn rig_model<'a>(parts: &'a [Part], issues: &[Issue]) -> Option<&'a Arc<Model>> {
    let root = parts.first()?;
    if !root.geometry.as_deref().is_some_and(is_creature_path)
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
    root.model.as_ref()
}

/// Anchors the item port of a skin part to the bone `helper` of the creature skeleton `bones`
/// (from [`skeleton_of`]): the bind pose of the bone in the space of `parent`, a skin part.
/// Returns `None` when `parent` is no skin part or the skeleton lacks the bone.
pub(crate) fn skin_port_anchor(
    bones: &[Bone],
    parent: &Part,
    helper: &str,
) -> Option<(Placement, Transform)> {
    if parent.placement != Placement::Skin {
        return None;
    }
    let pose = bone_pose(bones, helper)?;
    let transform = parent.world.inverse()?.then(&pose);
    Some((Placement::Helper(helper.to_owned()), transform))
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

/// Lists what the Loadout of a creature root could not read, such as an XML Loadout the Archive
/// lacks, so that a creature that draws nothing says why. Other Entities list nothing here.
pub(crate) fn note_loadout(root: &EntityNode, issues: &mut Vec<Issue>) {
    if !root
        .geometry()
        .is_some_and(|geometry| is_creature_path(geometry.path()))
    {
        return;
    }
    let notes = root
        .issues()
        .iter()
        .filter(|issue| {
            !matches!(
                issue,
                LoadoutError::AmbiguousClass { .. } | LoadoutError::VehicleImplementation { .. }
            )
        })
        .map(|issue| Issue {
            path: String::new(),
            name: root.class_name().to_owned(),
            kind: IssueKind::LoadoutUnreadable {
                reason: issue.to_string(),
            },
        });
    issues.extend(notes);
}

fn is_skinned(model: &Model, level: u32) -> bool {
    model
        .nearest_lod(level)
        .is_some_and(|lod| lod.meshes.iter().any(|mesh| mesh.influences.is_some()))
}
