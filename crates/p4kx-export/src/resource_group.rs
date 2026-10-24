// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_extraction::{Guid, RecordTree, Value};

use crate::contract::ResourceGroup;
use crate::guid_text::CatalogGuid;
use crate::record_fields::{class_name, elements, locale_key};
use crate::texts::Texts;

/// What the `ResourceTypeGroup` Records list: the group of each `ResourceType` and the parent of
/// each group.
#[derive(Debug, Default)]
pub(crate) struct Groups {
    by_resource: HashMap<Guid, CatalogGuid>,
    parents: HashMap<Guid, CatalogGuid>,
}

impl Groups {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Records what `listing` lists; the first group to list a `ResourceType` or a group wins.
    pub(crate) fn add(&mut self, listing: Listing) {
        let group = CatalogGuid(listing.group);
        for resource in listing.resources {
            self.by_resource.entry(resource).or_insert(group);
        }
        for child in listing.children {
            if child != listing.group {
                self.parents.entry(child).or_insert(group);
            }
        }
    }

    /// Returns the group that directly lists the `ResourceType` `resource`.
    pub(crate) fn of_resource(&self, resource: &Guid) -> Option<CatalogGuid> {
        self.by_resource.get(resource).copied()
    }

    /// Returns the group that lists the group `group`.
    pub(crate) fn parent_of(&self, group: &Guid) -> Option<CatalogGuid> {
        self.parents.get(group).copied()
    }
}

/// The `ResourceType` Records and the groups one `ResourceTypeGroup` lists.
#[derive(Debug)]
pub(crate) struct Listing {
    group: Guid,
    resources: Vec<Guid>,
    children: Vec<Guid>,
}

/// Returns what the `ResourceTypeGroup` tree `tree` lists in `resources` and `groups`.
pub(crate) fn listing(tree: &RecordTree<'_>) -> Listing {
    Listing {
        group: tree.guid(),
        resources: listed(tree, "resources"),
        children: listed(tree, "groups"),
    }
}

fn listed(tree: &RecordTree<'_>, field: &str) -> Vec<Guid> {
    elements(Some(tree.root()), field)
        .iter()
        .filter_map(|entry| match entry {
            Value::Reference(entry) if entry.location().is_none() => Some(entry.guid()),
            _ => None,
        })
        .collect()
}

/// Returns the resource group the `ResourceTypeGroup` tree `tree` describes, its name resolved by
/// `texts` and its parent taken from `groups`.
pub(crate) fn resource_group<'f>(
    tree: &RecordTree<'f>,
    texts: &Texts<'f>,
    groups: &Groups,
) -> ResourceGroup<'f> {
    ResourceGroup {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        name: texts.localised(locale_key(Some(tree.root()), "displayName")),
        parent_guid: groups.parent_of(&tree.guid()),
    }
}
