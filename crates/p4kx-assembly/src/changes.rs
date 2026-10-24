// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_dataforge::Guid;
use thiserror::Error;

use crate::merge::port_key;
use crate::{EntityLookupError, Incompatibility, ItemFacts, LoadoutError};

/// Own configuration of a Loadout: branches to leave out and Entities to attach instead.
///
/// A port is named by its path of port names from the root Entity, joined by `/` and matched
/// ignoring ASCII case, the path [`EntityNode::walk`](crate::EntityNode::walk) lists, e.g.
/// `hardpoint_weapon_nose/hardpoint_class_2`. Apply the changes with
/// [`LoadoutResolver::resolve_with`](crate::LoadoutResolver::resolve_with).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadoutChanges {
    exclusions: Vec<String>,
    replacements: Vec<(String, String)>,
}

impl LoadoutChanges {
    /// Creates changes that leave the Loadout as it is.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Leaves out what the port `port` holds, with everything attached below it.
    #[must_use]
    pub fn exclude(mut self, port: impl Into<String>) -> Self {
        self.exclusions.push(port.into());
        self
    }

    /// Attaches the Entity `entity` to the port `port` instead of what it holds. `entity` is a
    /// class name, a GUID or a record path, as [`Entities::find`](crate::Entities::find) takes
    /// it. The Entity brings its own Loadout; the Loadout of the Entity it replaces is dropped.
    #[must_use]
    pub fn replace(mut self, port: impl Into<String>, entity: impl Into<String>) -> Self {
        self.replacements.push((port.into(), entity.into()));
        self
    }

    /// Splits `text` of the form `<port>=<entity>` and adds it as a replacement.
    ///
    /// # Errors
    ///
    /// Returns [`ChangeError::InvalidReplacement`] when `text` lacks the `=` or names no port or
    /// no Entity.
    pub fn replace_text(self, text: &str) -> Result<Self, ChangeError> {
        let (port, entity) = split_replacement(text)?;
        Ok(self.replace(port, entity))
    }

    /// Returns whether the changes leave the Loadout as it is.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.exclusions.is_empty() && self.replacements.is_empty()
    }

    /// Returns the ports to leave out, as given.
    #[must_use]
    pub fn exclusions(&self) -> &[String] {
        &self.exclusions
    }

    /// Returns the replacements as `(port, entity)`, as given.
    #[must_use]
    pub fn replacements(&self) -> &[(String, String)] {
        &self.replacements
    }
}

fn split_replacement(text: &str) -> Result<(&str, &str), ChangeError> {
    text.split_once('=')
        .map(|(port, entity)| (port.trim(), entity.trim()))
        .filter(|(port, entity)| !port.is_empty() && !entity.is_empty())
        .ok_or_else(|| ChangeError::InvalidReplacement {
            text: text.to_owned(),
        })
}

/// Why changes to a Loadout cannot be applied.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ChangeError {
    /// A replacement is not of the form `<port>=<entity>`.
    #[error("{text:?} is not of the form <port>=<entity>")]
    InvalidReplacement {
        /// The text as given.
        text: String,
    },
    /// A port path is empty or has an empty segment.
    #[error("{path:?} is no port path: port names joined by '/'")]
    InvalidPort {
        /// The path as given.
        path: String,
    },
    /// The Loadout has no port of this path.
    #[error("the Loadout has no port {path}")]
    UnknownPort {
        /// The path as given.
        path: String,
    },
    /// Two changes contradict each other: one port is excluded and replaced, or a replacement
    /// lies in an excluded branch.
    #[error("the changes to {path} and {other} contradict each other")]
    Conflict {
        /// The path of the replacement.
        path: String,
        /// The path of the exclusion.
        other: String,
    },
    /// A port is replaced twice.
    #[error("the port {path} is replaced more than once")]
    Duplicate {
        /// The path as given.
        path: String,
    },
    /// The Entity a replacement names cannot be found.
    #[error("the replacement for {path}: {source}")]
    Entity {
        /// The path of the port.
        path: String,
        /// Why the Entity is not found.
        source: EntityLookupError,
    },
    /// The Entity a replacement names cannot be read.
    #[error("the replacement {entity} for {path} cannot be used: {source}")]
    Unreadable {
        /// The path of the port.
        path: String,
        /// The Entity as given.
        entity: String,
        /// Why it cannot be read.
        source: LoadoutError,
    },
    /// The Entity does not fit the port's types and sizes.
    #[error("{entity} does not fit the port {path}: {reason}")]
    Incompatible {
        /// The path of the port.
        path: String,
        /// The class name of the Entity.
        entity: String,
        /// What does not fit.
        reason: Incompatibility,
    },
    /// The Entity whose Loadout is resolved cannot be.
    #[error(transparent)]
    Loadout(#[from] LoadoutError),
}

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

fn under(key: &str, ancestor: &str) -> bool {
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

    /// Returns the first problem of the resolution: a rejected replacement, else a change that
    /// found no port.
    pub(crate) fn finish(self) -> Option<ChangeError> {
        let Self {
            exclusions,
            replacements,
            errors,
            ..
        } = self;
        errors.into_iter().next().or_else(|| {
            let mut unknown: Vec<String> = exclusions
                .into_values()
                .filter(|exclusion| !exclusion.matched)
                .map(|exclusion| exclusion.path)
                .chain(
                    replacements
                        .into_values()
                        .filter(|replacement| !replacement.matched)
                        .map(|replacement| replacement.path),
                )
                .collect();
            unknown.sort();
            unknown
                .into_iter()
                .next()
                .map(|path| ChangeError::UnknownPort { path })
        })
    }
}
