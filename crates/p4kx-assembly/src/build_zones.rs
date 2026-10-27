// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::BTreeSet;
use std::sync::Arc;

use p4kx_model::Model;

use crate::armour::{covered, uncovered};
use crate::build::{file_key, issue};
use crate::build_models::Loaded;
use crate::flatten::Draft;
use crate::node_body::body_of;
use crate::{BodyVariant, EntityNode, Issue};

/// The cut of a character's body: the zones its worn clothing covers, by the rule of the
/// character package.
pub(crate) struct BodyCut<'t> {
    variant: BodyVariant,
    skin: String,
    covered: BTreeSet<&'t str>,
}

impl<'t> BodyCut<'t> {
    /// Returns the cut for the body the head in `tree` names, from the clothing among `drafts`
    /// that has a model; `None` when the tree has no head or the clothing covers no zone.
    pub(crate) fn of(
        tree: &EntityNode,
        drafts: &[Draft<'t>],
        chosen: &[Option<&Loaded>],
    ) -> Option<Self> {
        let variant = body_of(tree.ports())?;
        let covered = covered(
            drafts
                .iter()
                .zip(chosen)
                .filter(|(_, found)| found.is_some())
                .flat_map(|(draft, _)| draft.node.zones.iter()),
        );
        (!covered.is_empty()).then(|| Self {
            variant,
            skin: file_key(variant.skin()),
            covered,
        })
    }

    /// Returns the model of `draft` as it is worn: the default body skin without the covered
    /// zones, reported in `issues`; any other model as it is.
    pub(crate) fn model_of(
        &self,
        draft: &Draft<'_>,
        loaded: &Loaded,
        issues: &mut Vec<Issue>,
    ) -> Arc<Model> {
        let model = Arc::clone(&loaded.model);
        if draft.model_key().as_deref() != Some(self.skin.as_str()) {
            return model;
        }
        let (model, kind) = uncovered(model, self.variant, &self.covered);
        issues.extend(kind.map(|kind| issue(draft, kind)));
        model
    }
}
