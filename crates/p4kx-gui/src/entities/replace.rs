// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_assembly::{EntityCatalogue, EntitySummary, PortConstraints};

use super::list::matches;

/// The choice of the Entity that replaces what a port holds: the Entities that fit the port,
/// narrowed by a search.
#[derive(Debug, Clone)]
pub struct ReplaceChooser {
    path: String,
    port: String,
    constraints: Option<PortConstraints>,
    catalogue: Arc<EntityCatalogue>,
    query: String,
    shown: Vec<u32>,
    fitting: usize,
}

impl ReplaceChooser {
    pub(super) fn new(
        catalogue: Arc<EntityCatalogue>,
        (path, port): (&str, &str),
        constraints: Option<PortConstraints>,
    ) -> Self {
        let mut chooser = Self {
            path: path.to_owned(),
            port: port.to_owned(),
            constraints,
            catalogue,
            query: String::new(),
            shown: Vec::new(),
            fitting: 0,
        };
        chooser.refilter();
        chooser
    }

    fn refilter(&mut self) {
        let terms: Vec<String> = self
            .query
            .split_whitespace()
            .map(str::to_ascii_lowercase)
            .collect();
        let entries = self.catalogue.entries();
        let fits: Vec<u32> = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                self.constraints
                    .as_ref()
                    .is_none_or(|constraints| constraints.check(entry.facts()).is_ok())
            })
            .filter_map(|(at, _)| u32::try_from(at).ok())
            .collect();
        self.fitting = fits.len();
        self.shown = fits
            .into_iter()
            .filter(|at| {
                usize::try_from(*at)
                    .ok()
                    .and_then(|at| entries.get(at))
                    .is_some_and(|entry| matches(entry, &terms))
            })
            .collect();
    }

    /// Returns the path of the port that gets the Entity.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the name of the port.
    #[must_use]
    pub fn port(&self) -> &str {
        &self.port
    }

    /// Returns what the port takes, or `None` when it takes any Entity.
    #[must_use]
    pub const fn constraints(&self) -> Option<&PortConstraints> {
        self.constraints.as_ref()
    }

    /// Returns the search text.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Searches the class names and record paths of the Entities that fit.
    pub fn set_query(&mut self, text: &str) {
        text.clone_into(&mut self.query);
        self.refilter();
    }

    /// Returns how many Entities fit the port, whatever the search says.
    #[must_use]
    pub const fn fitting(&self) -> usize {
        self.fitting
    }

    /// Returns how many of the Entities that fit the search leaves.
    #[must_use]
    pub const fn shown_len(&self) -> usize {
        self.shown.len()
    }

    /// Returns the Entity at `position` of the ones shown.
    #[must_use]
    pub fn get(&self, position: usize) -> Option<&EntitySummary> {
        let at = usize::try_from(*self.shown.get(position)?).ok()?;
        self.catalogue.entries().get(at)
    }
}
