// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::sync::Arc;

use p4kx_assembly::{EntityCatalogue, EntityKind, EntitySummary};
use p4kx_export::VariantGroups;
use p4kx_extraction::Guid;

use super::fold::Fold;

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

/// What a row of the picker stands for in its variant group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityRole {
    /// An Entity without variants.
    Single,
    /// The base of a group.
    Base {
        /// How many variants the base has.
        variants: usize,
        /// Whether the variants are listed below the base.
        expanded: bool,
    },
    /// A variant, listed below its base.
    Variant,
}

/// One row of the picker: an Entity and its place in a variant group.
#[derive(Debug, Clone, Copy)]
pub struct EntityRow<'a> {
    entity: &'a EntitySummary,
    role: EntityRole,
}

impl<'a> EntityRow<'a> {
    /// Returns the Entity of the row.
    #[must_use]
    pub const fn entity(&self) -> &'a EntitySummary {
        self.entity
    }

    /// Returns the place of the Entity in its variant group.
    #[must_use]
    pub const fn role(&self) -> EntityRole {
        self.role
    }
}

#[derive(Debug, Clone, Copy)]
struct Row {
    entry: u32,
    role: EntityRole,
    forced: bool,
}

/// The Entities of the picker: the ships, vehicles and weapons of the catalogue that the filter
/// and the search text leave, with the variants of a base folded under it.
///
/// A search lists a base that matches or has a matching variant and opens a group with a matching
/// variant, listing only the variants that match.
#[derive(Debug, Clone)]
pub struct EntityList {
    catalogue: Arc<EntityCatalogue>,
    fold: Fold,
    rows: Vec<Row>,
    matched: usize,
    counts: [usize; 3],
    filter: EntityFilter,
    terms: Vec<String>,
    open: HashSet<Guid>,
    shut: HashSet<Guid>,
    selected: Option<Guid>,
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
            fold: Fold::new(&catalogue, None),
            catalogue,
            rows: Vec::new(),
            matched: 0,
            counts,
            filter: EntityFilter::All,
            terms: Vec::new(),
            open: HashSet::new(),
            shut: HashSet::new(),
            selected: None,
        };
        list.refilter(EntityFilter::All, "");
        list
    }

    pub(super) fn refilter(&mut self, filter: EntityFilter, query: &str) {
        let terms: Vec<String> = query
            .split_whitespace()
            .map(str::to_ascii_lowercase)
            .collect();
        if terms != self.terms {
            self.shut.clear();
        }
        self.filter = filter;
        self.terms = terms;
        self.rebuild();
    }

    fn rebuild(&mut self) {
        let mut rows = Vec::new();
        let mut matched = 0;
        for position in 0..self.catalogue.entries().len() {
            if !self.listed(position) {
                continue;
            }
            matched += usize::from(self.hit(position));
            if !self
                .fold
                .parent(position)
                .is_some_and(|base| self.listed(base))
            {
                self.push_group(&mut rows, position);
            }
        }
        self.rows = rows;
        self.matched = matched;
    }

    fn listed(&self, position: usize) -> bool {
        self.catalogue
            .entries()
            .get(position)
            .is_some_and(|entry| self.filter.takes(entry.kind()))
    }

    fn hit(&self, position: usize) -> bool {
        self.catalogue
            .entries()
            .get(position)
            .is_some_and(|entry| matches(entry, &self.terms))
    }

    fn listed_child(&self, child: u32) -> Option<usize> {
        usize::try_from(child)
            .ok()
            .filter(|position| self.listed(*position))
    }

    fn is_selected(&self, child: u32) -> bool {
        self.selected.is_some_and(|selected| {
            usize::try_from(child)
                .ok()
                .and_then(|at| self.catalogue.entries().get(at))
                .is_some_and(|entry| entry.guid() == selected)
        })
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one decision after another about a single group"
    )]
    fn push_group(&self, rows: &mut Vec<Row>, position: usize) {
        let variants: Vec<u32> = self
            .fold
            .children(position)
            .iter()
            .copied()
            .filter(|child| self.listed_child(*child).is_some())
            .collect();
        let searching = !self.terms.is_empty();
        let hits = variants
            .iter()
            .filter(|child| self.listed_child(**child).is_some_and(|at| self.hit(at)))
            .count();
        let forced = searching && hits > 0;
        let (Ok(own), Some(guid)) = (
            u32::try_from(position),
            self.catalogue
                .entries()
                .get(position)
                .map(EntitySummary::guid),
        ) else {
            return;
        };
        if !self.hit(position) && hits == 0 {
            return;
        }
        let holds_selection = variants.iter().any(|child| self.is_selected(*child));
        let expanded = holds_selection
            || if forced {
                !self.shut.contains(&guid)
            } else {
                self.open.contains(&guid)
            };
        let only_hits = searching && hits > 0;
        let role = if variants.is_empty() {
            EntityRole::Single
        } else {
            EntityRole::Base {
                variants: variants.len(),
                expanded,
            }
        };
        rows.push(Row {
            entry: own,
            role,
            forced,
        });
        if expanded {
            rows.extend(
                variants
                    .iter()
                    .filter(|child| {
                        !only_hits
                            || self.is_selected(**child)
                            || self.listed_child(**child).is_some_and(|at| self.hit(at))
                    })
                    .map(|child| Row {
                        entry: *child,
                        role: EntityRole::Variant,
                        forced: false,
                    }),
            );
        }
    }

    /// Folds the variants of `groups` under their bases.
    pub(super) fn set_groups(&mut self, groups: &VariantGroups) {
        self.fold = Fold::new(&self.catalogue, Some(groups));
        self.rebuild();
    }

    /// Tells the list which Entity is selected: a group that holds it stays open, with the
    /// selected variant listed.
    pub(super) fn set_selected(&mut self, selected: Option<Guid>) {
        self.selected = selected;
        self.rebuild();
    }

    /// Returns the catalogue the list is taken from.
    #[must_use]
    pub const fn catalogue(&self) -> &Arc<EntityCatalogue> {
        &self.catalogue
    }

    /// Returns how many rows the list shows.
    #[must_use]
    pub const fn shown_len(&self) -> usize {
        self.rows.len()
    }

    /// Returns how many Entities of the shown kinds match the search, those folded under a base
    /// included.
    #[must_use]
    pub const fn matched_len(&self) -> usize {
        self.matched
    }

    /// Returns the Entity at row `position`.
    #[must_use]
    pub fn get(&self, position: usize) -> Option<&EntitySummary> {
        self.row(position).map(|row| row.entity)
    }

    /// Returns the row at `position`.
    #[must_use]
    pub fn row(&self, position: usize) -> Option<EntityRow<'_>> {
        let row = self.rows.get(position)?;
        let at = usize::try_from(row.entry).ok()?;
        Some(EntityRow {
            entity: self.catalogue.entries().get(at)?,
            role: row.role,
        })
    }

    /// Lists the variants of the base at row `position` below it, or folds them away again;
    /// nothing changes for a row that is no base.
    pub fn toggle(&mut self, position: usize) {
        let Some(row) = self.rows.get(position).copied() else {
            return;
        };
        let EntityRole::Base { expanded, .. } = row.role else {
            return;
        };
        let Some(guid) = self.row(position).map(|row| row.entity.guid()) else {
            return;
        };
        let (set, other) = if row.forced {
            (&mut self.shut, &mut self.open)
        } else {
            (&mut self.open, &mut self.shut)
        };
        other.remove(&guid);
        if expanded == row.forced {
            set.insert(guid);
        } else {
            set.remove(&guid);
        }
        self.rebuild();
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
