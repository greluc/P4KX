// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_assembly::EntityCatalogue;
use p4kx_export::VariantGroups;

/// Which Entities of the catalogue are variants of which base, as positions in the catalogue.
#[derive(Debug, Clone, Default)]
pub(super) struct Fold {
    parent: Vec<Option<u32>>,
    children: Vec<Vec<u32>>,
}

impl Fold {
    /// Returns the folds of `groups` over the Entities of `catalogue`; a variant whose base is
    /// not in the catalogue stays on its own.
    pub(super) fn new(catalogue: &EntityCatalogue, groups: Option<&VariantGroups>) -> Self {
        let entries = catalogue.entries();
        let mut fold = Self {
            parent: vec![None; entries.len()],
            children: vec![Vec::new(); entries.len()],
        };
        let Some(groups) = groups.filter(|groups| !groups.is_empty()) else {
            return fold;
        };
        let at: HashMap<_, _> = entries
            .iter()
            .enumerate()
            .filter_map(|(position, entry)| Some((entry.guid(), u32::try_from(position).ok()?)))
            .collect();
        for (position, entry) in entries.iter().enumerate() {
            let Some(base) = groups
                .base_of(entry.guid())
                .and_then(|base| at.get(&base).copied())
            else {
                continue;
            };
            let Ok(own) = u32::try_from(position) else {
                continue;
            };
            if let Some(slot) = fold.parent.get_mut(position) {
                *slot = Some(base);
            }
            if let Some(kids) = usize::try_from(base)
                .ok()
                .and_then(|base| fold.children.get_mut(base))
            {
                kids.push(own);
            }
        }
        fold
    }

    /// Returns the position of the base the Entity at `position` is a variant of.
    pub(super) fn parent(&self, position: usize) -> Option<usize> {
        let base = self.parent.get(position).copied().flatten()?;
        usize::try_from(base).ok()
    }

    /// Returns the positions of the variants of the Entity at `position`, in catalogue order.
    pub(super) fn children(&self, position: usize) -> &[u32] {
        self.children.get(position).map_or(&[], Vec::as_slice)
    }
}
