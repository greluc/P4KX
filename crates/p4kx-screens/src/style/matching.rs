// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_dataforge::{DataForge, Guid, TAG_PATH_SEPARATOR, TagTree};

use super::entry::{Condition, Entry};

#[derive(Debug, Clone)]
pub(crate) struct Subject {
    pub(crate) kind: String,
    pub(crate) tags: Vec<Guid>,
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
        }
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

    /// Returns the tag at `path`, ignoring case.
    pub(crate) fn guid_of(&self, path: &str) -> Option<Guid> {
        self.paths
            .iter()
            .find(|(_, own)| own.eq_ignore_ascii_case(path))
            .map(|(guid, _)| *guid)
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
        Condition::Ancestor(inner) => {
            for at in (0..above.len()).rev() {
                if let Some(ancestor) = above.get(at) {
                    let rest = above.get(..at).unwrap_or_default();
                    if all(inner, ancestor, rest, tags, budget) {
                        return true;
                    }
                }
                if *budget == 0 {
                    return false;
                }
            }
            false
        }
        Condition::AllOf(inner) => all(inner, subject, above, tags, budget),
        Condition::AnyOf(inner) => any(inner, subject, above, tags, budget),
        Condition::Unsupported(_) => false,
    }
}
