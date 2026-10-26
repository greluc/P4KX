// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_assembly::{EntityCatalogue, EntityKind, EntitySummary, LoadoutChanges};
use p4kx_extraction::Guid;
use p4kx_screens::{EntityScreens, ScreenComposer};

use super::job::Job;
use crate::EntryIndex;
use crate::entities::{ResolveFailure, resolve};
use crate::forge_cache::ReadFailure;

/// The ships and ground vehicles of the DataForge that the search text leaves.
#[derive(Debug, Clone)]
pub struct ShipList {
    catalogue: Arc<EntityCatalogue>,
    taken: Vec<u32>,
    shown: Vec<u32>,
}

impl ShipList {
    pub(super) fn new(catalogue: Arc<EntityCatalogue>) -> Self {
        let taken = catalogue
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, entry)| matches!(entry.kind(), EntityKind::Ship | EntityKind::Vehicle))
            .filter_map(|(position, _)| u32::try_from(position).ok())
            .collect();
        let mut list = Self {
            catalogue,
            taken,
            shown: Vec::new(),
        };
        list.refilter("");
        list
    }

    pub(super) fn is_of(&self, catalogue: &Arc<EntityCatalogue>) -> bool {
        Arc::ptr_eq(&self.catalogue, catalogue)
    }

    pub(super) fn refilter(&mut self, query: &str) {
        let terms: Vec<String> = query
            .split_whitespace()
            .map(str::to_ascii_lowercase)
            .collect();
        self.shown = self
            .taken
            .iter()
            .copied()
            .filter(|&at| {
                usize::try_from(at)
                    .ok()
                    .and_then(|at| self.catalogue.entries().get(at))
                    .is_some_and(|entry| matches(entry, &terms))
            })
            .collect();
    }

    /// Returns how many ships and vehicles the DataForge holds.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.taken.len()
    }

    /// Returns how many the search leaves.
    #[must_use]
    pub const fn shown_len(&self) -> usize {
        self.shown.len()
    }

    /// Returns the ship or vehicle at `position` of the shown ones.
    #[must_use]
    pub fn get(&self, position: usize) -> Option<&EntitySummary> {
        let at = usize::try_from(*self.shown.get(position)?).ok()?;
        self.catalogue.entries().get(at)
    }
}

fn matches(entry: &EntitySummary, terms: &[String]) -> bool {
    let (name, path) = (
        entry.class_name().to_ascii_lowercase(),
        entry.record_path().to_ascii_lowercase(),
    );
    terms
        .iter()
        .all(|term| name.contains(term.as_str()) || path.contains(term.as_str()))
}

/// Resolves the default Loadout of the Entity `guid` and lists the screens it shows, on a
/// background thread.
pub(super) fn list_screens(
    index: &Arc<EntryIndex>,
    guid: Guid,
    wake: impl Fn() + Send + 'static,
) -> Result<Job<Result<EntityScreens, ResolveFailure>>, String> {
    let index = Arc::clone(index);
    Job::spawn("ui-screens", wake, move || {
        let tree = resolve(&index, guid, &LoadoutChanges::default(), &|| false)?.into_node();
        let source = index
            .forge()
            .read(&index, || {})
            .map_err(|failure| match failure {
                ReadFailure::ArchiveChanged(_) => ResolveFailure::ArchiveChanged,
                ReadFailure::Unreadable(reason) => ResolveFailure::Failed(reason),
            })?;
        let forge = source
            .entry
            .open()
            .map_err(|error| ResolveFailure::Failed(crate::model::describe(&error)))?;
        Ok(ScreenComposer::new(&forge).screens_of(&tree))
    })
}
