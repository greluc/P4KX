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
) -> bool {
    entry.lists.is_empty()
        || entry.lists.iter().any(|list| {
            list.iter()
                .all(|condition| holds(condition, subject, above, tags))
        })
}

fn holds(condition: &Condition, subject: &Subject, above: &[Subject], tags: &TagPaths) -> bool {
    match condition {
        Condition::Tag(tag) => tags.has(subject, *tag),
        Condition::NotTag(tag) => !tags.has(subject, *tag),
        Condition::AnyOfTag(wanted) => wanted.iter().any(|tag| tags.has(subject, *tag)),
        Condition::AllOfTag(wanted) => wanted.iter().all(|tag| tags.has(subject, *tag)),
        Condition::Type(kind) => subject.kind == *kind,
        Condition::Parent(inner) => above
            .split_last()
            .is_some_and(|(parent, rest)| inner.iter().all(|c| holds(c, parent, rest, tags))),
        Condition::Ancestor(inner) => (0..above.len()).rev().any(|at| {
            above.get(at).is_some_and(|ancestor| {
                let rest = above.get(..at).unwrap_or_default();
                inner.iter().all(|c| holds(c, ancestor, rest, tags))
            })
        }),
        Condition::AllOf(inner) => inner.iter().all(|c| holds(c, subject, above, tags)),
        Condition::AnyOf(inner) => inner.iter().any(|c| holds(c, subject, above, tags)),
        Condition::Unsupported(_) => false,
    }
}
