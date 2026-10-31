// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The parts of a Loadout tree that another worn Entity hides by its `HiddenParts`.

use crate::armour::{Facts, hidden_by};
use crate::build::issue;
use crate::flatten::{Draft, Flat};
use crate::{IssueKind, PortNode};

/// Returns what decides whether `draft` is hidden: the name of the port it sits at, and the ports
/// its Entity hides. The root and a landing gear sit at no port and hide nothing.
fn facts_of<'t>(draft: &'t Draft<'_>) -> Facts<'t> {
    let hides = if draft.gear.is_none() {
        draft.node.hidden_ports()
    } else {
        &[]
    };
    Facts::at(&draft.name, draft.port.map(PortNode::name), hides)
}

/// Returns the name of the port a port path ends in.
fn port_of(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, port)| port)
}

/// Marks the parts that another part's Record hides as hidden, by the rules of the Armour
/// (ADR-0279): a port is matched by name at any depth, ignoring case, and every part hides by
/// its Record whether it is hidden itself or not. A part below a hidden part is hidden too.
///
/// A hidden part is reported as [`IssueKind::PartHidden`] and has no model, so it covers no zone
/// of the body. An attachment that could not be resolved at a hidden port is reported as hidden
/// instead of unresolved.
pub(crate) fn hide_worn(flat: &mut Flat<'_>) {
    let drafts = flat.drafts.len();
    let unresolved: Vec<usize> = flat
        .issues
        .iter()
        .enumerate()
        .filter(|(_, found)| matches!(found.kind, IssueKind::Unresolved(_)))
        .map(|(at, _)| at)
        .collect();
    let by = {
        let mut facts: Vec<Facts<'_>> = flat.drafts.iter().map(facts_of).collect();
        facts.extend(unresolved.iter().filter_map(|at| {
            let port = port_of(&flat.issues.get(*at)?.path);
            Some(Facts::at(port, Some(port), &[]))
        }));
        hidden_by(&facts)
    };
    if by.iter().all(Option::is_none) {
        return;
    }
    let (of_drafts, of_unresolved) = by.split_at(drafts);
    hide_drafts(flat, of_drafts);
    for (at, hider) in unresolved.into_iter().zip(of_unresolved) {
        let hider = hider.and_then(|hider| flat.drafts.get(hider));
        if let (Some(hider), Some(found)) = (hider, flat.issues.get_mut(at)) {
            let port = port_of(&found.path).to_owned();
            found.kind = IssueKind::PartHidden {
                port,
                by: hider.name.clone(),
            };
        }
    }
}

/// Hides the drafts `by` names a hider for, and the drafts below them, and reports each part that
/// sits at a port.
fn hide_drafts(flat: &mut Flat<'_>, by: &[Option<usize>]) {
    let mut hiders: Vec<Option<usize>> = Vec::with_capacity(by.len());
    for (index, own) in by.iter().enumerate() {
        let inherited = flat
            .drafts
            .get(index)
            .and_then(|draft| draft.parent)
            .and_then(|parent| hiders.get(parent).copied().flatten());
        let hider = own.or(inherited);
        hiders.push(hider);
        let Some(hider) = hider else { continue };
        let by_name = flat.drafts.get(hider).map(|draft| draft.name.clone());
        let Some(draft) = flat.drafts.get_mut(index).filter(|draft| !draft.hidden) else {
            continue;
        };
        draft.hidden = true;
        if let (Some(port), Some(by)) = (draft.port, by_name) {
            let kind = IssueKind::PartHidden {
                port: port.name().to_owned(),
                by,
            };
            flat.issues.push(issue(draft, kind));
        }
    }
}
