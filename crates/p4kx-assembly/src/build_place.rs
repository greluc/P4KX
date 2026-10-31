// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{Bone, Model, Transform};

use crate::build::issue;
use crate::build_models::Loaded;
use crate::creature_rig;
use crate::flatten::Draft;
use crate::placement::{bone_transform, node_transform, offset};
use crate::{Issue, IssueKind, Part, Placement, PortDefinition, PortNode, PortTransform};

/// Returns whether the parent of the part is a skin part.
pub(crate) fn under_skin(draft: &Draft<'_>, parts: &[Part]) -> bool {
    draft
        .parent
        .and_then(|parent| parts.get(parent))
        .is_some_and(|parent| parent.placement == Placement::Skin)
}

/// Returns the transform of a part to its parent's and how it was found. `rig` is the skeleton of a
/// creature root that the item ports of its skins anchor to.
pub(crate) fn place(
    draft: &Draft<'_>,
    parts: &[Part],
    model: Option<&Loaded>,
    rig: Option<&[Bone]>,
    issues: &mut Vec<Issue>,
) -> (Transform, Placement) {
    let mut unreported = Vec::new();
    let issues = if draft.hidden {
        &mut unreported
    } else {
        issues
    };
    let Some(parent) = draft.parent.and_then(|parent| parts.get(parent)) else {
        return (Transform::IDENTITY, Placement::Root);
    };
    if let Some(gear) = draft.gear {
        return place_gear(draft, gear.bone(), parent, issues);
    }
    let Some(port) = draft.port else {
        return (Transform::IDENTITY, Placement::Origin);
    };
    let port_definition = port.definition();
    if port_definition.is_some_and(PortDefinition::is_skin) {
        return (Transform::IDENTITY, Placement::Skin);
    }
    let definition = port_definition.and_then(PortDefinition::transform);
    let helper = definition
        .map(PortTransform::helper)
        .filter(|helper| !helper.is_empty());
    let (mut placement, mut base) = anchor(parent.model(), helper, port);
    if !matches!(placement, Placement::Helper(_))
        && let Some((found, transform)) = rig
            .zip(helper)
            .and_then(|(rig, helper)| creature_rig::skin_port_anchor(rig, parent, helper))
    {
        (placement, base) = (found, transform);
    }
    if let Some(helper) = helper
        && placement == Placement::Origin
        && parent.model().is_some()
    {
        let helper = helper.to_owned();
        issues.push(issue(draft, IssueKind::HelperNotFound { helper }));
    }
    let Some(definition) = definition else {
        return (base, placement);
    };
    let mut local = base.then(&offset(definition.position(), definition.rotation()));
    let inner = (!definition.offset_helper().is_empty())
        .then(|| {
            let child = model?;
            node_transform(&child.model, definition.offset_helper())?.inverse()
        })
        .flatten();
    if let Some(inner) = inner {
        local = local.then(&inner);
    }
    (local, placement)
}

/// Returns the transform of a landing gear: the node `bone` of its Entity's model.
fn place_gear(
    draft: &Draft<'_>,
    bone: &str,
    parent: &Part,
    issues: &mut Vec<Issue>,
) -> (Transform, Placement) {
    let Some(parent_model) = parent.model() else {
        return (Transform::IDENTITY, Placement::Origin);
    };
    if let Some(transform) = node_transform(parent_model, bone) {
        return (transform, Placement::Helper(bone.to_owned()));
    }
    let helper = bone.to_owned();
    issues.push(issue(draft, IssueKind::HelperNotFound { helper }));
    (Transform::IDENTITY, Placement::Origin)
}

/// Finds the node of the parent's model a port is anchored to: its helper as a node or as a bone
/// of the skeleton, else a node named like the port.
fn anchor(parent: Option<&Model>, helper: Option<&str>, port: &PortNode) -> (Placement, Transform) {
    let Some(parent) = parent else {
        return (Placement::Origin, Transform::IDENTITY);
    };
    if let Some(helper) = helper
        && let Some(transform) =
            node_transform(parent, helper).or_else(|| bone_transform(parent, helper))
    {
        return (Placement::Helper(helper.to_owned()), transform);
    }
    node_transform(parent, port.name())
        .map_or((Placement::Origin, Transform::IDENTITY), |transform| {
            (Placement::PortNode, transform)
        })
}
