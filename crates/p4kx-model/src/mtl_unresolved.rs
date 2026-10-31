// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The layer files that the material libraries of a scene could not resolve, counted once each.

use std::collections::HashSet;

use crate::MaterialStatus;

/// The layer files missing, unreadable or over the load budget across the material libraries of
/// one scene, see [`MaterialLibrary::unresolved_layers`](crate::MaterialLibrary::unresolved_layers).
///
/// A file that several libraries name counts once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnresolvedLayers {
    first: Option<String>,
    seen: HashSet<String>,
}

impl UnresolvedLayers {
    /// Collects the unresolved layer files of `statuses`, in their order.
    #[must_use]
    pub fn of<'a>(statuses: impl IntoIterator<Item = &'a MaterialStatus>) -> Self {
        let mut all = Self::default();
        all.extend(statuses);
        all
    }

    /// Adds the unresolved layer files of every status of `statuses`.
    pub fn extend<'a>(&mut self, statuses: impl IntoIterator<Item = &'a MaterialStatus>) {
        let libraries = statuses.into_iter().filter_map(MaterialStatus::library);
        for path in libraries.flat_map(|library| &library.unresolved_layers) {
            if self.seen.insert(path.clone()) {
                self.first.get_or_insert_with(|| path.clone());
            }
        }
    }

    /// Returns how many distinct layer files are unresolved.
    #[must_use]
    pub fn count(&self) -> usize {
        self.seen.len()
    }

    /// Returns the first unresolved layer file, as the layer names it.
    #[must_use]
    pub fn first(&self) -> Option<&str> {
        self.first.as_deref()
    }
}
