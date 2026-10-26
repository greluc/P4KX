// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_dataforge::Guid;

use crate::merge::port_key;
use crate::{ChangeError, Incompatibility, ItemFacts, LoadoutChanges};

/// What to do with a port.
pub(crate) enum Action {
    Keep,
    Exclude,
    Replace(Guid, ItemFacts),
}

struct Exclusion {
    path: String,
    matched: bool,
}

struct Replacement {
    path: String,
    guid: Guid,
    facts: ItemFacts,
    matched: bool,
}

/// The changes of one resolution with the path of the port being built.
pub(crate) struct Plan {
    exclusions: HashMap<String, Exclusion>,
    replacements: HashMap<String, Replacement>,
    errors: Vec<ChangeError>,
    key: String,
    marks: Vec<usize>,
    stopped: Vec<String>,
}

/// Returns the key of a port path, or `None` when it is no path.
pub(crate) fn path_key(path: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() || path.split('/').any(|segment| segment.trim().is_empty()) {
        return None;
    }
    Some(
        path.split('/')
            .map(|segment| port_key(segment.trim()))
            .collect::<Vec<_>>()
            .join("/"),
    )
}

pub(crate) fn under(key: &str, ancestor: &str) -> bool {
    key.strip_prefix(ancestor)
        .is_some_and(|rest| rest.starts_with('/'))
}

impl Plan {
    /// Checks the paths of the changes against each other and returns the plan; `facts` is
    /// called for each replacement with its Entity text and returns its GUID and facts.
    pub(crate) fn new(
        changes: &LoadoutChanges,
        mut facts: impl FnMut(&str, &str) -> Result<(Guid, ItemFacts), ChangeError>,
    ) -> Result<Self, ChangeError> {
        let mut exclusions: HashMap<String, Exclusion> = HashMap::new();
        for path in changes.exclusions() {
            let key =
                path_key(path).ok_or_else(|| ChangeError::InvalidPort { path: path.clone() })?;
            exclusions.entry(key).or_insert_with(|| Exclusion {
                path: path.trim().to_owned(),
                matched: false,
            });
        }
        let keys: Vec<String> = exclusions.keys().cloned().collect();
        exclusions.retain(|key, _| !keys.iter().any(|other| under(key, other)));
        let mut replacements: HashMap<String, Replacement> = HashMap::new();
        for (path, entity) in changes.replacements() {
            let key =
                path_key(path).ok_or_else(|| ChangeError::InvalidPort { path: path.clone() })?;
            if let Some(other) = exclusions
                .iter()
                .find(|(other, _)| **other == key || under(&key, other))
                .map(|(_, exclusion)| exclusion.path.clone())
            {
                return Err(ChangeError::Conflict {
                    path: path.trim().to_owned(),
                    other,
                });
            }
            if replacements.contains_key(&key) {
                return Err(ChangeError::Duplicate {
                    path: path.trim().to_owned(),
                });
            }
            let (guid, item) = facts(path.trim(), entity)?;
            replacements.insert(
                key,
                Replacement {
                    path: path.trim().to_owned(),
                    guid,
                    facts: item,
                    matched: false,
                },
            );
        }
        Ok(Self {
            exclusions,
            replacements,
            errors: Vec::new(),
            key: String::new(),
            marks: Vec::new(),
            stopped: Vec::new(),
        })
    }

    /// Steps into the port `name` of the Entity being built.
    pub(crate) fn enter(&mut self, name: &str) {
        self.marks.push(self.key.len());
        if !self.key.is_empty() {
            self.key.push('/');
        }
        self.key.push_str(&port_key(name));
    }

    /// Steps out of the port entered last.
    pub(crate) fn leave(&mut self) {
        if let Some(length) = self.marks.pop() {
            self.key.truncate(length);
        }
    }

    /// Returns what to do with the port entered last, marking the change as found.
    pub(crate) fn action(&mut self) -> Action {
        if let Some(exclusion) = self.exclusions.get_mut(&self.key) {
            exclusion.matched = true;
            return Action::Exclude;
        }
        if let Some(replacement) = self.replacements.get_mut(&self.key) {
            replacement.matched = true;
            return Action::Replace(replacement.guid, replacement.facts.clone());
        }
        Action::Keep
    }

    /// Returns the path as given of the replacement at the port entered last.
    pub(crate) fn path_of_replacement(&self) -> String {
        self.replacements
            .get(&self.key)
            .map_or_else(|| self.key.clone(), |replacement| replacement.path.clone())
    }

    /// Records that the Entity does not fit the port entered last.
    pub(crate) fn reject(&mut self, entity: &ItemFacts, reason: Incompatibility) {
        self.errors.push(ChangeError::Incompatible {
            path: self.path_of_replacement(),
            entity: entity.class_name().to_owned(),
            reason,
        });
    }

    /// Records that the Entity at the port entered last is a docked vehicle that is not
    /// expanded, so that changes below it find no port and are no error.
    pub(crate) fn stopped_here(&mut self) {
        self.stopped.push(self.key.clone());
    }

    /// Records that the replacement of the port entered last is the root or an ancestor of it.
    pub(crate) fn reject_cycle(&mut self, entity: &str) {
        self.errors.push(ChangeError::Cycle {
            path: self.path_of_replacement(),
            entity: entity.to_owned(),
        });
    }

    /// Returns the paths of the changes that lie below a docked vehicle that is not expanded and
    /// so were not applied, or the first problem of the resolution: a rejected replacement, else
    /// a change that found no port.
    pub(crate) fn finish(self) -> Result<Vec<String>, ChangeError> {
        let Self {
            exclusions,
            replacements,
            errors,
            stopped,
            ..
        } = self;
        if let Some(error) = errors.into_iter().next() {
            return Err(error);
        }
        let unreachable = |key: &str| stopped.iter().any(|docked| under(key, docked));
        let unmatched = exclusions
            .into_iter()
            .filter(|(_, exclusion)| !exclusion.matched)
            .map(|(key, exclusion)| (key, exclusion.path))
            .chain(
                replacements
                    .into_iter()
                    .filter(|(_, replacement)| !replacement.matched)
                    .map(|(key, replacement)| (key, replacement.path)),
            );
        let (mut ignored, mut unknown) = (Vec::new(), Vec::new());
        for (key, path) in unmatched {
            if unreachable(&key) {
                ignored.push(path);
            } else {
                unknown.push(path);
            }
        }
        unknown.sort();
        if let Some(path) = unknown.into_iter().next() {
            return Err(ChangeError::UnknownPort { path });
        }
        ignored.sort();
        Ok(ignored)
    }
}
