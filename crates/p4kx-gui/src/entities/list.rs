// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_assembly::{EntityCatalogue, EntityKind, EntitySummary};

/// Which Entities the picker lists.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum EntityFilter {
    /// Ships, vehicles and weapons.
    #[default]
    All,
    /// Ships only.
    Ships,
    /// Ground vehicles only.
    Vehicles,
    /// Weapons only.
    Weapons,
}

impl EntityFilter {
    /// Every filter, in the order the picker offers them.
    pub const ALL: [Self; 4] = [Self::All, Self::Ships, Self::Vehicles, Self::Weapons];

    const fn takes(self, kind: EntityKind) -> bool {
        matches!(
            (self, kind),
            (
                Self::All,
                EntityKind::Ship | EntityKind::Vehicle | EntityKind::Weapon
            ) | (Self::Ships, EntityKind::Ship)
                | (Self::Vehicles, EntityKind::Vehicle)
                | (Self::Weapons, EntityKind::Weapon)
        )
    }
}

/// The Entities of the picker: the ships, vehicles and weapons of the catalogue that the filter
/// and the search text leave.
#[derive(Debug, Clone)]
pub struct EntityList {
    catalogue: Arc<EntityCatalogue>,
    shown: Vec<u32>,
    counts: [usize; 3],
}

impl EntityList {
    pub(super) fn new(catalogue: Arc<EntityCatalogue>) -> Self {
        let mut counts = [0; 3];
        for entry in catalogue.entries() {
            if let Some(slot) = slot(entry.kind()).and_then(|slot| counts.get_mut(slot)) {
                *slot += 1;
            }
        }
        let mut list = Self {
            catalogue,
            shown: Vec::new(),
            counts,
        };
        list.refilter(EntityFilter::All, "");
        list
    }

    pub(super) fn refilter(&mut self, filter: EntityFilter, query: &str) {
        let terms: Vec<String> = query
            .split_whitespace()
            .map(str::to_ascii_lowercase)
            .collect();
        self.shown = self
            .catalogue
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, entry)| filter.takes(entry.kind()) && matches(entry, &terms))
            .filter_map(|(position, _)| u32::try_from(position).ok())
            .collect();
    }

    /// Returns the catalogue the list is taken from.
    #[must_use]
    pub const fn catalogue(&self) -> &Arc<EntityCatalogue> {
        &self.catalogue
    }

    /// Returns how many Entities the list shows.
    #[must_use]
    pub const fn shown_len(&self) -> usize {
        self.shown.len()
    }

    /// Returns the Entity at `position` of the shown ones.
    #[must_use]
    pub fn get(&self, position: usize) -> Option<&EntitySummary> {
        let at = usize::try_from(*self.shown.get(position)?).ok()?;
        self.catalogue.entries().get(at)
    }

    /// Returns how many ships, vehicles and weapons the catalogue holds, whatever the search
    /// says.
    #[must_use]
    pub fn total(&self, filter: EntityFilter) -> usize {
        match filter {
            EntityFilter::All => self.counts.iter().sum(),
            EntityFilter::Ships => self.counts.first().copied().unwrap_or(0),
            EntityFilter::Vehicles => self.counts.get(1).copied().unwrap_or(0),
            EntityFilter::Weapons => self.counts.get(2).copied().unwrap_or(0),
        }
    }
}

const fn slot(kind: EntityKind) -> Option<usize> {
    match kind {
        EntityKind::Ship => Some(0),
        EntityKind::Vehicle => Some(1),
        EntityKind::Weapon => Some(2),
        _ => None,
    }
}

/// Returns whether every term, in lower case, occurs in the class name or the record path of the
/// Entity, ignoring ASCII case.
pub(super) fn matches(entry: &EntitySummary, terms: &[String]) -> bool {
    terms
        .iter()
        .all(|term| contains(entry.class_name(), term) || contains(entry.record_path(), term))
}

fn contains(haystack: &str, term: &str) -> bool {
    let (haystack, term) = (haystack.as_bytes(), term.as_bytes());
    term.is_empty()
        || haystack
            .windows(term.len())
            .any(|window| window.eq_ignore_ascii_case(term))
}
