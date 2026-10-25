// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_dataforge::{DataForge, Guid, TAG_PATH_SEPARATOR, TagTree};

use super::condition::{Condition, counted};
use super::entry::Entry;

/// A widget as the selectors see it: its type, its tags, its place among its siblings and, where
/// an entry asks for them, its children.
#[derive(Debug, Clone)]
pub(crate) struct Subject {
    pub(crate) kind: String,
    pub(crate) tags: Vec<Guid>,
    pub(crate) position: usize,
    pub(crate) kids: Vec<Self>,
}

impl Subject {
    pub(crate) fn of(type_name: &str, tags: &[Guid]) -> Self {
        let name = type_name
            .strip_prefix("BuildingBlocks_")
            .unwrap_or(type_name);
        let kind = if name == "DisplayWidget" {
            "Base"
        } else {
            name.strip_prefix("Widget").unwrap_or(name)
        };
        Self {
            kind: kind.to_owned(),
            tags: tags.to_vec(),
            position: 0,
            kids: Vec::new(),
        }
    }

    /// Sets the place of the widget among its siblings.
    pub(crate) const fn at(mut self, position: usize) -> Self {
        self.position = position;
        self
    }

    /// Sets the children `Child` selectors look at.
    pub(crate) fn with_kids(mut self, kids: Vec<Self>) -> Self {
        self.kids = kids;
        self
    }
}

#[derive(Debug, Default)]
pub(crate) struct TagPaths {
    paths: HashMap<Guid, String>,
}

impl TagPaths {
    pub(crate) fn build(forge: &DataForge<'_>) -> Self {
        let tree = TagTree::build(forge);
        Self {
            paths: tree
                .iter()
                .map(|tag| (tag.guid(), tag.path().to_owned()))
                .collect(),
        }
    }

    fn within(&self, tag: Guid, ancestor: Guid) -> bool {
        if tag == ancestor {
            return true;
        }
        let (Some(path), Some(base)) = (self.paths.get(&tag), self.paths.get(&ancestor)) else {
            return false;
        };
        path.strip_prefix(base.as_str())
            .is_some_and(|rest| rest.starts_with(TAG_PATH_SEPARATOR))
    }

    fn has(&self, subject: &Subject, wanted: Guid) -> bool {
        subject.tags.iter().any(|tag| self.within(*tag, wanted))
    }
}

pub(crate) fn applies(
    entry: &Entry,
    subject: &Subject,
    above: &[Subject],
    tags: &TagPaths,
    budget: &mut u64,
) -> bool {
    *budget = budget.saturating_sub(1);
    entry.lists.is_empty()
        || entry
            .lists
            .iter()
            .any(|list| all(list, subject, above, tags, budget))
}

fn all(
    conditions: &[Condition],
    subject: &Subject,
    above: &[Subject],
    tags: &TagPaths,
    budget: &mut u64,
) -> bool {
    conditions
        .iter()
        .all(|condition| holds(condition, subject, above, tags, budget))
}

fn any(
    conditions: &[Condition],
    subject: &Subject,
    above: &[Subject],
    tags: &TagPaths,
    budget: &mut u64,
) -> bool {
    conditions
        .iter()
        .any(|condition| holds(condition, subject, above, tags, budget))
}

/// Looks for an ancestor that meets `conditions`, nearest first, without looking at or beyond
/// one that meets `breaks`.
fn ancestor(
    (conditions, breaks): (&[Condition], &[Condition]),
    above: &[Subject],
    tags: &TagPaths,
    budget: &mut u64,
) -> bool {
    for at in (0..above.len()).rev() {
        if let Some(candidate) = above.get(at) {
            let rest = above.get(..at).unwrap_or_default();
            if !breaks.is_empty() && all(breaks, candidate, rest, tags, budget) {
                return false;
            }
            if all(conditions, candidate, rest, tags, budget) {
                return true;
            }
        }
        if *budget == 0 {
            return false;
        }
    }
    false
}

fn holds(
    condition: &Condition,
    subject: &Subject,
    above: &[Subject],
    tags: &TagPaths,
    budget: &mut u64,
) -> bool {
    if *budget == 0 {
        return false;
    }
    *budget -= 1;
    match condition {
        Condition::Tag(tag) => tags.has(subject, *tag),
        Condition::NotTag(tag) => !tags.has(subject, *tag),
        Condition::AnyOfTag(wanted) => wanted.iter().any(|tag| tags.has(subject, *tag)),
        Condition::AllOfTag(wanted) => wanted.iter().all(|tag| tags.has(subject, *tag)),
        Condition::Type(kind) => subject.kind == *kind,
        Condition::Parent(inner) => above
            .split_last()
            .is_some_and(|(parent, rest)| all(inner, parent, rest, tags, budget)),
        Condition::Ancestor { conditions, breaks } => {
            ancestor((conditions, breaks), above, tags, budget)
        }
        Condition::Child(inner) => subject.kids.iter().any(|kid| {
            *budget > 0 && {
                *budget -= 1;
                all(inner, kid, &[], tags, budget)
            }
        }),
        Condition::Count {
            kind,
            count,
            offset,
        } => counted(*kind, *count, *offset, subject.position),
        Condition::AllOf(inner) => all(inner, subject, above, tags, budget),
        Condition::AnyOf(inner) => any(inner, subject, above, tags, budget),
        Condition::Unsupported(_) => false,
    }
}
