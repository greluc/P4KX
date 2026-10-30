// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::BTreeSet;
use std::sync::Arc;

use p4kx_model::Model;

use crate::armour::{covered, uncovered};
use crate::build::issue;
use crate::build_models::Loaded;
use crate::flatten::Draft;
use crate::node_body::body_of;
use crate::{BodyVariant, EntityNode, Issue, IssueKind};

const BODY_TYPE: &str = "Char_Body";

fn is_body(draft: &Draft<'_>) -> bool {
    draft.node.attach_type().eq_ignore_ascii_case(BODY_TYPE)
}

/// The cut of a character's body: the zones its worn Entities cover, by the rule of the
/// character package.
pub(crate) struct BodyCut<'t> {
    variant: BodyVariant,
    covered: BTreeSet<&'t str>,
}

impl<'t> BodyCut<'t> {
    /// Returns the cut for the body the head in `tree` names, from the Entities among `drafts`
    /// that have a model; `None` when the tree has no head or the Entities cover no zone.
    ///
    /// A cut that finds no `Char_Body` part with a model is reported in `issues` as
    /// [`IssueKind::BodyZonesUnknown`] and not made.
    pub(crate) fn of(
        tree: &EntityNode,
        drafts: &[Draft<'t>],
        chosen: &[Option<&Loaded>],
        issues: &mut Vec<Issue>,
    ) -> Option<Self> {
        let variant = body_of(tree.ports())?;
        let covered = covered(
            drafts
                .iter()
                .zip(chosen)
                .filter(|(_, found)| found.is_some())
                .flat_map(|(draft, _)| draft.node.zones.iter()),
        );
        if covered.is_empty() {
            return None;
        }
        let has_body = drafts
            .iter()
            .zip(chosen)
            .any(|(draft, found)| found.is_some() && is_body(draft));
        if !has_body {
            let root = drafts.first()?;
            issues.push(issue(
                root,
                IssueKind::BodyZonesUnknown(format!("no part with a model is a {BODY_TYPE}")),
            ));
            return None;
        }
        Some(Self { variant, covered })
    }

    /// Returns the model of `draft` as it is worn: a `Char_Body` part without the covered zones,
    /// reported in `issues`; any other model as it is.
    pub(crate) fn model_of(
        &self,
        draft: &Draft<'_>,
        loaded: &Loaded,
        issues: &mut Vec<Issue>,
    ) -> Arc<Model> {
        let model = Arc::clone(&loaded.model);
        if !is_body(draft) {
            return model;
        }
        let (model, kind) = uncovered(model, self.variant, &self.covered);
        issues.extend(kind.map(|kind| issue(draft, kind)));
        model
    }
}
