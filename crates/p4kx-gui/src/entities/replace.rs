// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_assembly::{EntityCatalogue, EntitySummary, PortConstraints};

use super::list::matches;

/// The choice of the Entity that replaces what a port holds: every Entity the search leaves, the
/// ones that fit the port first and the others marked.
#[derive(Debug, Clone)]
pub struct ReplaceChooser {
    path: String,
    port: String,
    constraints: Option<PortConstraints>,
    catalogue: Arc<EntityCatalogue>,
    query: String,
    shown: Vec<(u32, bool)>,
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
        let (mut fits, mut misfits) = (Vec::new(), Vec::new());
        for (at, entry) in entries.iter().enumerate() {
            let Ok(at) = u32::try_from(at) else {
                break;
            };
            let fit = self
                .constraints
                .as_ref()
                .is_none_or(|constraints| constraints.check(entry.facts()).is_ok());
            if fit {
                fits.push(at);
            } else if matches(entry, &terms) {
                misfits.push((at, false));
            }
        }
        self.fitting = fits.len();
        let mut shown: Vec<(u32, bool)> = fits
            .into_iter()
            .filter(|at| {
                usize::try_from(*at)
                    .ok()
                    .and_then(|at| entries.get(at))
                    .is_some_and(|entry| matches(entry, &terms))
            })
            .map(|at| (at, true))
            .collect();
        shown.append(&mut misfits);
        self.shown = shown;
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

    /// Searches the class names and record paths of the Entities.
    pub fn set_query(&mut self, text: &str) {
        text.clone_into(&mut self.query);
        self.refilter();
    }

    /// Returns how many Entities fit the port, whatever the search says.
    #[must_use]
    pub const fn fitting(&self) -> usize {
        self.fitting
    }

    /// Returns how many Entities the search leaves, the ones that fit first.
    #[must_use]
    pub const fn shown_len(&self) -> usize {
        self.shown.len()
    }

    /// Returns the Entity at `position` of the ones shown.
    #[must_use]
    pub fn get(&self, position: usize) -> Option<&EntitySummary> {
        let (at, _) = *self.shown.get(position)?;
        self.catalogue.entries().get(usize::try_from(at).ok()?)
    }

    /// Returns whether the Entity at `position` of the ones shown fits the port.
    #[must_use]
    pub fn fits(&self, position: usize) -> bool {
        self.shown.get(position).is_some_and(|(_, fit)| *fit)
    }
}
